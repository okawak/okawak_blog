# OCI Public IP固定化とTerraform変更計画

## 目的

OCI上の既存VPSを再作成せず、現在のEphemeral Public IPをReserved Public IPへ置き換えます。Reserved Public IPをTerraformのdesired stateへ含め、SSH接続先を固定した後、Issue #118でCloudflare Tunnel移行とorigin直接公開の閉鎖へ進みます。

この文書はrepository ownerが`terraform/oci/`を変更、review、plan、applyするための計画です。Codexを含む通常のrepository作業では`terraform/`をread-onlyとし、この文書の作成時点ではTerraform file、state、OCI resourceを変更していません。

関連資料:

- [OCI Terraform Provider: `oci_core_public_ip`](https://docs.oracle.com/en-us/iaas/tools/terraform-provider-oci/latest/docs/r/core_public_ip.html)
- [OCI Terraform Provider: Resource Discovery](https://docs.oracle.com/en-us/iaas/Content/dev/terraform/resource-discovery.htm)
- [Issue #118](https://github.com/okawak/okawak_blog/issues/118)

## 現行構成

`terraform/oci/`には次のresourceが定義されています。

- `module.network`
  - compartment
  - VCN
  - internet gateway
  - route table
  - security list
  - subnet
- `module.compute`
  - Oracle Linux compute instance
  - primary VNIC

compute instanceの`create_vnic_details.assign_public_ip`は`true`で、primary private IPへEphemeral Public IPを自動割り当てする構成です。Reserved Public IPを明示するresourceはありません。

root moduleは`oracle/oci`を要求していますが、現行の`compute`と`network` child moduleには`required_providers`がありません。Terraformはsourceを省略したproviderを後方互換の`hashicorp/<local name>`として解釈するため、child moduleと既存stateはlegacy `hashicorp/oci`を参照しています。Public IP変更前に、OCI公式の[legacy provider source移行手順](https://docs.oracle.com/en-us/iaas/Content/dev/terraform/migrating-legacy.htm)に従って`oracle/oci`へ統一します。

security listはSSH、HTTP、HTTPSなどをPublic Internetへ許可しています。HTTP/HTTPSの閉鎖はCloudflare Tunnelが安定稼働した後のPhase OCI-Dで行い、Public IP固定化と同じapplyへ混ぜません。

## 必須の安全条件

- compute instance、boot volume、VNIC、private IPを再作成しない
- compartment、VCN、subnet、route tableを再作成しない
- `terraform plan`に想定外の`destroy`または`replace`が1件でもあればapplyしない
- provider upgrade、module再編、命名整理をPublic IP移行へ混ぜない
- state、plan file、`terraform.tfvars`をcommitまたは共有しない
- Reserved Public IPを作成してからEphemeral Public IPを外す
- Cloudflare DNSを更新するまで一時的なWeb停止が起きる前提で作業時間を確保する
- OCI Console ConnectionをPublic IPに依存しないbreak-glass経路として準備する
- 旧AWS IAM access keyはReserved Public IPとreboot検証が終わるまで`Inactive`のまま削除しない

## Phase OCI-0: stateと実体の確認

### 0.1 作業開始前の確認

管理端末でTerraform認証が有効であることを確認します。secret、private key、stateの内容は貼り付けません。

```bash
cd terraform/oci
terraform version
terraform providers
terraform state list
```

少なくとも次のresource addressがstateに存在することを確認します。

```text
module.compute.oci_core_instance.oraclelinux_instance
module.network.oci_identity_compartment.my_compartment
module.network.oci_core_vcn.my_vcn
module.network.oci_core_internet_gateway.my_internet_gateway
module.network.oci_core_route_table.my_route_table
module.network.oci_core_security_list.my_security_list
module.network.oci_core_subnet.my_subnet
```

存在しないresourceがあっても、すぐに`apply`しません。OCI ConsoleのOCIDと照合し、既存resourceをimportします。Terraform 1.5以降ではreview可能な`import` blockを優先できます。OCIのResource Discoveryは比較資料として使用できますが、生成HCLをそのまま既存moduleへ上書きしません。

### 0.2 state backup

現在のbackendを確認し、apply前にstateを安全な場所へbackupします。local stateの場合の例です。

```bash
install -m 0600 terraform.tfstate \
  "terraform.tfstate.before-reserved-ip.$(date +%Y%m%d%H%M%S)"
```

backupにもcredentialやresource属性が含まれる可能性があります。repository外の暗号化された保管先へ移し、不要になったcopyは安全に削除します。

### 0.3 provider sourceの正規化

Public IPのHCLを追加する前に、activeな各child moduleへprovider requirementを追加します。provider configurationはrootから継承しますが、source requirementはmoduleごとに宣言する必要があります。

`compute/versions.tf`と`network/versions.tf`の例:

```hcl
terraform {
  required_providers {
    oci = {
      source = "oracle/oci"
    }
  }
}
```

comment outされている`database` moduleは現時点のconfiguration graphへ含まれません。再度有効化する前に同じ`required_providers`を追加します。

現在のlocal lockfileにはOCI provider `6.32.0`から`8.23.0`への更新差分があります。最新版を採用する判断は維持しますが、Public IP変更へ混ぜず、このPhaseでprovider source正規化とmajor version更新だけを完了させます。root moduleでは採用versionを明示し、将来の`terraform init`で意図せず更新されないようにします。

```hcl
terraform {
  required_providers {
    oci = {
      source  = "oracle/oci"
      version = "8.23.0"
    }
  }
}
```

```bash
terraform init

terraform state replace-provider \
  'registry.terraform.io/hashicorp/oci' \
  'registry.terraform.io/oracle/oci'
```

`terraform state replace-provider`は確認promptを読み、`-auto-approve`を付けません。このcommandはOCI resourceを変更せずstate内のprovider addressを置き換え、変更前stateのbackupを自動作成します。0.2の手動backupも維持します。

実行後、configurationとstateの両方が`oracle/oci`だけを参照することを確認します。

```bash
terraform providers
terraform state list
terraform plan -refresh-only
```

期待する結果:

- root、`module.compute`、`module.network`が`registry.terraform.io/oracle/oci`
- stateが`registry.terraform.io/oracle/oci`
- provider versionがroot requirementとlockfileの両方で`8.23.0`
- provider migrationによるOCI resourceのadd、change、destroy、replaceがない
- `.terraform.lock.hcl`にlegacy `hashicorp/oci`が残っていない

legacy `hashicorp/oci`がconfigurationまたはstateに残っている間は、Reserved Public IPの実装へ進みません。

OCI providerのmajor version更新直後は、computed attributeやnested blockの正規化だけがrefresh-only差分に出る場合があります。`local_volume_size_in_gbs = 0`やcomputed CIDR listの削除は、通常planでremote updateにならなければstate表現の更新として扱えます。

route tableの`route_rules`削除が表示された場合は、refresh-only planをapplyする前にOCI ConsoleまたはOCI CLIでlive routeを確認します。少なくとも次のdefault routeが存在することを確認します。

```text
destination: 0.0.0.0/0
destination type: CIDR_BLOCK
target: managed internet gateway
route type: STATIC
```

続けて通常の`terraform plan`を実行します。最終summaryが`No changes`で、live routeも上記と一致する場合だけproviderによるstate正規化と判断します。通常planがroute ruleの削除、再追加、instance/VNICのreplacementを提案する場合はapplyせず、provider upgrade差分を先に解決します。

live routeが存在し、通常planが`No changes`になった場合は、確認済みのrefresh-only planだけをapplyしてstate表現を更新します。通常の`terraform apply`は実行しません。

```bash
terraform plan -refresh-only -out=plan_refresh
terraform show -no-color plan_refresh
terraform apply plan_refresh
terraform plan
```

`terraform show`でproviderによるcomputed attributeとnested blockの正規化だけであることを再確認してから、保存したplanをapplyします。最後の通常planが再び`No changes`になれば、provider source正規化とmajor version更新は完了です。`plan_refresh`にはstate相当の情報が含まれ得るため、commitや共有はせず、確認後に安全に削除します。

### 0.4 refresh-only plan

HCL変更前にlive OCIとの差分を確認します。

```bash
terraform plan -refresh-only -out=plan_refresh
terraform show -no-color plan_refresh
```

`plan_refresh`をcommitまたはIssueへ添付しません。次をOCI Consoleと照合します。

- instance OCID、shape、OCPU、memory
- primary VNIC OCID
- primary private IPとそのOCID
- 現在のEphemeral Public IP
- subnet、security list、route table
- boot volumeのpreserve方針

既存driftがある場合はPublic IP変更と分離し、どちらをdesired stateにするか決めてから先へ進みます。

## 最終的なTerraform設計

### Primary private IPの参照

compute moduleからinstanceのprivate IP addressをoutputし、subnetとaddressでprimary private IP objectを参照します。

```hcl
// compute/outputs.tf
output "private_ip" {
  value = oci_core_instance.oraclelinux_instance.private_ip
}

// root module
data "oci_core_private_ips" "blog_primary" {
  ip_address = module.compute.private_ip
  subnet_id  = module.network.subnet_id
}

locals {
  blog_primary_private_ip_id = one([
    for private_ip in data.oci_core_private_ips.blog_primary.private_ips :
    private_ip.id
    if private_ip.is_primary
  ])
}
```

実装時はlive環境でprimary private IPが一意に選ばれることを`terraform console`またはplanで確認します。listのindex `0`を無条件に使わず、`is_primary`である条件を明示します。

### Reserved Public IP

Reserved Public IPはcompute instanceとは独立したroot resourceとして管理し、instance交換時にも保持できる境界にします。

最終形の例:

```hcl
resource "oci_core_public_ip" "blog" {
  compartment_id = module.network.compartment_id
  display_name    = "okawak_blog_public_ip"
  lifetime        = "RESERVED"
  private_ip_id   = local.blog_primary_private_ip_id

  lifecycle {
    prevent_destroy = true
  }
}
```

`private_ip_id`はupdate可能ですが、割当先private IPに別のPublic IPが付いているとOCI APIは失敗します。そのため、以下の段階applyを必須にします。

compute moduleの最終形では次のようにEphemeral Public IPの自動作成を止めます。

```hcl
create_vnic_details {
  assign_public_ip = false
  subnet_id        = var.subnet_id
}
```

root outputはinstanceのcomputed Public IPではなく、Reserved Public IP resourceを参照します。

```hcl
output "public-ip-for-compute-instance" {
  value = oci_core_public_ip.blog.ip_address
}

output "public-ip-ocid" {
  value = oci_core_public_ip.blog.id
}
```

既存のmodule outputを残すか削除するかは別のformat整理にせず、この変更内で参照先を一意にします。

## Phase OCI-A: Reserved Public IPを未割当で作成

最初のapplyでは`oci_core_public_ip.blog`へ`private_ip_id`を設定しません。既存通信はEphemeral Public IPのまま維持します。

```hcl
resource "oci_core_public_ip" "blog" {
  compartment_id = module.network.compartment_id
  display_name    = "okawak_blog_public_ip"
  lifetime        = "RESERVED"

  lifecycle {
    prevent_destroy = true
  }
}
```

期待するplan:

```text
Plan: 1 to add, 0 to change, 0 to destroy.
```

apply後、Reserved Public IPのOCIDとIP addressを記録します。この時点ではCloudflare DNSやSSH接続先を変更しません。

## Phase OCI-B: Ephemeral Public IPを外す

compute moduleの`assign_public_ip`だけを`false`へ変更します。Reserved Public IPにはまだ`private_ip_id`を設定しません。

apply前に次を準備します。

- Terraformを実行する管理端末がVPSのSSH sessionへ依存せず、OCI APIへ接続できる
- OCI Consoleへlogin済みで、instance、primary VNIC、private IPを確認できる
- Cloudflare Dashboardへlogin済みで、対象A recordをすぐ更新できる
- 現在のEphemeral Public IP、作成済みReserved Public IP、primary private IPを記録済み
- Phase OCI-CのHCL変更と確認commandを手元で参照できる
- 現在のSSH sessionは閉じず、切断される前提で別terminalから作業する

旧Ephemeral Public IPは外した後に同じaddressへ戻せません。Phase OCI-Bのplanは保存して内容をreviewし、Phase OCI-Cを続けて実行できる時間帯だけapplyします。

停止時間を短くするため、Phase OCI-Bをapplyする前に「Primary private IPの参照」で示したcompute output、`oci_core_private_ips` data source、primary private IPを一意に選ぶlocalを追加します。この時点では`oci_core_public_ip.blog.private_ip_id`を追加しません。data sourceの参照追加はOCI resourceを変更しませんが、configurationを変更した後は以前保存したplanを使わず、Phase OCI-Bのplanを作り直します。

期待するplan:

- compute instanceまたはprimary VNICのin-place updateのみ
- `0 to destroy`
- instance、VNIC、boot volumeのreplacementなし

plan summaryは`0 to add, 1 to change, 0 to destroy`相当を期待します。`-/+`、`must be replaced`、Reserved Public IPの削除または割当が含まれる場合はapplyしません。

`replace`が表示された場合はapplyせず、provider schemaとstate driftを再調査します。

applyすると現在のSSHとorigin通信が切れます。OCI Consoleと次のPhase OCI-Cを実行できる管理端末を開いた状態で進めます。

## Phase OCI-C: Reserved Public IPを割り当てる

`oci_core_public_ip.blog`へ次を追加します。

```hcl
private_ip_id = local.blog_primary_private_ip_id
```

期待するplan:

- Reserved Public IPのin-place update
- `0 to add, 1 to change, 0 to destroy`相当
- compute instanceのreplacementなし

apply後、Public IPのstateが`ASSIGNED`になることを確認します。

```bash
terraform state show oci_core_public_ip.blog
terraform output public-ip-for-compute-instance
```

Cloudflare DNS record一覧で旧Ephemeral Public IPを検索し、そのIPを直接参照しているproxied Aレコードを新しいReserved Public IPへ更新します。`www`がapex `okawak.net`を参照するCNAMEならapexのAレコード変更だけで追従します。`www`が独立したAレコードの場合は`www`も更新します。あわせて旧IPを参照するDNS-only record、wildcard record、不要なAAAA recordがないことを確認します。Cloudflare resourceは現行OCI Terraformの管理対象に含めず、Cloudflare providerの導入判断はIssue #118で分離します。

canonical originは`https://www.okawak.net`なので、apexと`www`の両方を確認します。cache済みpageだけでなく、queryを付けたreadiness endpointでも新originへの到達を確認します。

```bash
curl --fail --output /dev/null --write-out 'apex %{http_code}\n' \
  https://okawak.net/
curl --fail --output /dev/null --write-out 'www %{http_code}\n' \
  https://www.okawak.net/
curl --fail -H 'Cache-Control: no-cache' \
  "https://www.okawak.net/api/ready?cutover=$(date +%s)"
```

新しいIPでSSH接続し、次を確認します。

```bash
sudo systemctl is-active okawak_blog nginx
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
curl --fail --output /dev/null \
  --write-out 'HTTPS %{http_code}\n' \
  https://www.okawak.net/
```

最後に通常planが差分なしになることを確認します。

```bash
terraform plan
```

## Phase OCI-D: rebootとCloudflare Tunnel

Reserved Public IP固定後にVPSをrebootし、同じPublic IPでSSHへ再接続できること、applicationとnginxが自動起動することを確認します。

reboot検証後、Issue #118でCloudflare Tunnelを導入します。Tunnelが安定稼働するまでは80/443のsecurity ruleを削除しません。Tunnel経由のhome、category、article、static asset、WASMを確認した後、別planでOCIの80/443 ingressを閉じます。

SSHは当面Reserved Public IPを使用します。SSHのCloudflare Access移行は、日常操作性、`scp`/`rsync`、障害時のbreak-glass経路を検証してから別途判断します。

## Rollback

### Phase OCI-A

Reserved Public IPは未割当なので既存通信へ影響しません。問題を修正して再planします。`prevent_destroy`を外して削除する操作は、IPを不要と判断した場合だけ行います。

### Phase OCI-BからOCI-Cの間

Ephemeral Public IPは外すと同じaddressを復元できません。新しいEphemeral IPを作って戻すより、作成済みReserved Public IPの割当失敗を修正することを優先します。

Terraformによる割当が長時間失敗し、復旧を急ぐ場合はOCI Consoleで作成済みReserved Public IPをprimary private IPへ割り当てます。その後、Terraform stateとHCLをrefreshし、差分ゼロへ戻します。手動割当を未記録のまま放置しません。

### Reserved Public IP割当後

Cloudflare DNSを旧IPへ戻しても、旧Ephemeral Public IPは復元されません。rollback先は割当済みReserved Public IPです。Cloudflare、SSH config、監視先をReserved Public IPへ統一します。

## 受け入れ条件

- existing OCI resourceとTerraform stateが対応している
- compute instance、VNIC、boot volumeのreplacementなしで移行できる
- Reserved Public IPが`oci_core_public_ip`としてstate管理される
- final planが`0 to add, 0 to change, 0 to destroy`になる
- SSH接続先がReserved Public IPで固定される
- Cloudflare経由のHTTPS、health、readinessが成功する
- reboot後もPublic IPが変わらず、systemd servicesが自動復旧する
- state、plan、credentialがrepositoryやIssueへ漏れていない
- Cloudflare Tunnelと80/443閉鎖がPublic IP移行と分離されている
