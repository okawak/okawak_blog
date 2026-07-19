# OCI network

## 現行構成

VPSはTerraform管理のReserved Public IPを持ちます。public IPの固定化によって、reboot後もSSH接続先を維持します。web trafficはCloudflare Tunnelを通るため、originの80/443 ingressは開けません。

OCI networkとcompute resourceは`terraform/oci`で管理し、Cloudflare Tunnel、Published application、DNSはCloudflare Dashboardで管理します。

## 管理端末からの初期構築

OCI ConsoleでTerraform用userとAPI signing keyを用意し、管理端末のOCI CLIで認証を確認します。private key、fingerprint、OCID、`terraform.tfvars`を共有しません。

```bash
oci iam region list --output table
```

`terraform/oci/terraform.tfvars`へ次を設定します。

- `tenancy_ocid`
- `region`
- `user_ocid`
- `fingerprint`
- `private_key_path`
- `ssh_public_key_path`
- `source_id`: 使用するOracle Linux image OCID

管理端末から初期化、plan review、applyを行います。

```bash
cd terraform/oci
terraform init
terraform providers
terraform validate
terraform fmt -check -recursive
terraform plan -out=plan_deploy
terraform show -no-color plan_deploy
terraform apply plan_deploy
terraform plan
```

configurationとstateのproviderが`registry.terraform.io/oracle/oci`で、最後のplanが`No changes`であることを確認します。保存済みplanとlocal stateはsecret相当として扱い、repository外の暗号化された保管先へbackupします。

初回applyではcompartment、VCN、internet gateway、route table、security list、subnet、compute instance、Reserved Public IPを作成します。instance、VNIC、boot volumeのreplaceが出た場合は理由を確認せずapplyしません。

## Security list

現行のsecurity listは次の通信だけを許可します。

### Ingress

- TCP 22: 新規VPSのbootstrap用。OCIでは許可するが、通常時のsshdはLISTENしない
- TCP 60022: SSH
- UDP 51820: 既存のWireGuard用途
- ICMP type 3/code 4: path MTU discovery

TCP 80、443は許可しません。TCP 22は初期構築を単純にするためsecurity listでは維持しますが、通常時のsshdはTCP 60022だけでLISTENします。

ICMP echo requestは許可しません。外部からの`ping`が失敗しても想定どおりです。

### Egress

- UDP 7844: Cloudflare TunnelのQUIC
- TCP: outbound traffic

`cloudflared`はQUICが使えない場合にHTTPSへfallbackできますが、通常経路としてUDP 7844を明示的に許可します。

## Reserved Public IP

Reserved Public IPはprimary private IPへ割り当て、Terraform stateで管理します。削除事故を避けるため`prevent_destroy`を維持します。

初回構築時はcompute instanceをPublic IPなしで作成し、そのprimary private IPへReserved Public IPを割り当てます。`assign_public_ip=false`を維持し、Ephemeral Public IPを併用しません。

確認時はOCI CLIで次を照合します。

```bash
oci compute instance list-vnics \
  --instance-id '<INSTANCE_OCID>' \
  --query 'data[0].{private_ip:"private-ip",public_ip:"public-ip",vnic_id:id}' \
  --output table

oci network public-ip get \
  --public-ip-id '<RESERVED_PUBLIC_IP_OCID>' \
  --query 'data.{ip:"ip-address",lifetime:lifetime,"private-ip-id":"private-ip-id"}' \
  --output table
```

Reserved Public IPの`private-ip-id`がVPSのprimary private IP resourceと一致し、実際のpublic IPがTerraform outputおよびSSH接続先と一致することを確認します。

## Terraform変更時の確認

repository ownerが`terraform/oci`を変更し、管理端末で実行します。通常のagent作業ではこのdirectoryをread-onlyとして扱います。

```bash
terraform init
terraform validate
terraform plan
```

planはresource address単位で確認します。network変更では次を特に確認します。

- compute instanceやReserved Public IPのreplaceまたはdestroyがない
- primary private IPとの割り当てが維持される
- bootstrap用のTCP 22 ingressが維持される
- TCP 60022のSSH経路を誤って閉じない
- web originのTCP 80/443を再公開しない
- UDP 7844 egressを維持する

apply後に再度planを実行し、`No changes`を確認します。SSH関連の変更では既存sessionを残したまま、別sessionでTCP 60022への接続を確認します。

## 検証

管理端末から次を確認します。

```bash
ssh -p 60022 '<USER>@<RESERVED_PUBLIC_IP>'
curl --fail https://www.okawak.net/api/ready
curl --fail https://okawak.net/api/ready
```

VPSではserviceを確認します。

```bash
sudo systemctl is-active okawak_blog cloudflared
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

rebootを伴う確認ではReserved Public IPが変わらず、両serviceが`enabled`かつ`active`で、Cloudflare DashboardのTunnelが`Healthy`へ戻ることを確認します。
