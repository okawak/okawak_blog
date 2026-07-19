# 本番環境の初期構築

## 目的

新しい環境で`okawak_blog`を構築するときの実行順序を示します。個々のcommandと更新手順はリンク先のrunbookを一次情報とします。

実行場所を次の3つに分けます。

- 管理端末: Terraform、AWS CLI、OCI CLI、certificate作成、SSH/SCP
- VPS: application、IAM Roles Anywhere helper、systemd、`cloudflared`
- Cloudflare Dashboard: zone、Tunnel、Published application、DNS

secret、private key、Terraform state、保存済みplan、Tunnel tokenをGit、Issue、PR、チャットへ記録しません。

## 1. 管理端末の認証を確認する

Terraform、AWS CLI、OCI CLI、OpenSSL、SSHの導入手順はこの文書の対象外です。管理端末に設定済みの管理者credentialが対象accountを指すことだけを確認します。

```bash
aws sts get-caller-identity
oci iam region list --output table
```

## 2. AWS resourceとruntime identityを構築する

AWS Terraform backendのS3 bucketとKMS aliasが先に必要です。現行configurationではbackend bucket自体を管理せず、KMS keyは同じroot moduleにあるため、完全な初回だけはbootstrapとimportが必要です。既存環境では再作成しません。

backendを確認した後、CA certificateを管理端末で作成し、`terraform/aws/terraform.tfvars`へ公開CA certificateのpathなどを設定します。backendとAWS resourceは[AWS Terraform](./aws-terraform.md)、certificateとVPS認証は[AWS runtime認証](./aws-runtime-auth.md)に従います。

```bash
cd terraform/aws
terraform init
terraform validate
terraform plan -out=plan_deploy
terraform show -no-color plan_deploy
terraform apply plan_deploy
terraform plan
```

最後のplanが`No changes`であることを確認します。`force_destroy`、IAM policy、KMS、IAM access keyに関するplanは特に慎重に確認します。

## 3. OCI networkとVPSを構築する

`terraform/oci/terraform.tfvars`へtenancy、user、API key、SSH public key、Oracle Linux imageの値を設定します。[OCI network](./oci-network.md)に従い、管理端末からplanをreviewしてapplyします。

```bash
cd terraform/oci
terraform init
terraform providers
terraform validate
terraform plan -out=plan_deploy
terraform show -no-color plan_deploy
terraform apply plan_deploy
terraform plan
```

Reserved Public IPを取得します。security listはbootstrap用のTCP 22と通常運用用のTCP 60022を許可します。新規VPSの初期設定後にsshdを60022へ設定し、通常時は60022だけでLISTENさせます。SSHはCloudflare Tunnelへ移さず、日常運用と保守作業の経路として維持します。

```bash
terraform output public-ip-for-compute-instance
ssh -p 60022 'okawak@<RESERVED_PUBLIC_IP>'
```

OCI Terraformは現在local stateを使います。stateとbackupをrepository外の暗号化された保管先へ保存します。

## 4. VPSへapplicationを配置する

VPSのOS、SSH、一般的なbuild tool、運用userの準備手順はこの文書の対象外です。repositoryが`/opt/okawak_blog`にあり、[runtime serviceのVPS build tool override](../../service/README.md#vps-build-tool-override)が有効であることを前提とします。

IAM Roles Anywhereのhelper、AWS config、client certificate、private keyを先に配置します。[AWS runtime認証](./aws-runtime-auth.md)のVPS手順でcaller identityとS3 readを確認します。

applicationをbuildしてserviceを配置します。

```bash
cd /opt/okawak_blog
mise run versions-check
mise run build-project
mise run production-deploy

sudo systemctl enable okawak_blog
sudo systemctl is-enabled okawak_blog
sudo systemctl is-active okawak_blog
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

`production-deploy`はapplication serviceだけを更新します。Cloudflare Tunnelは独立したserviceとして維持します。

## 5. Cloudflare Tunnelを構築する

Cloudflare Dashboardでzoneが有効であることを確認し、remote-managed Tunnel `okawak-blog-vps`を作成します。tokenは安全にVPSへ入力し、`/etc/cloudflared/token`以外へ保存しません。

[Cloudflare Tunnel](./cloudflare-tunnel.md)に従い、VPSへRPMとrepository管理のunitを配置してから、Dashboardで次を作成します。

- `okawak.net` -> `http://127.0.0.1:8008`
- `www.okawak.net` -> `http://127.0.0.1:8008`

DNS targetにはconnectorのReplica IDではなくTunnel IDを使います。apex CNAMEはCloudflareのCNAME flatteningを使います。

## 6. 構築完了を確認する

### VPS

```bash
sudo systemctl is-enabled okawak_blog cloudflared
sudo systemctl is-active okawak_blog cloudflared
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
sudo journalctl -u okawak_blog -u cloudflared --since '30 minutes ago' --no-pager
```

### 管理端末

```bash
ssh -p 60022 'okawak@<RESERVED_PUBLIC_IP>'
curl --fail https://okawak.net/api/ready
curl --fail https://www.okawak.net/api/ready
```

### Cloudflare Dashboard

- Tunnelが`Healthy`
- connectorが接続済み
- apexと`www`のPublished applicationが同じlocalhost originを指す
- DNSがProxiedでTunnel IDを指す

ブラウザでhome、category、article、CSS、client-side navigationを確認し、consoleにWASM初期化errorがないことを確認します。

## 7. 定常運用

- application deploy: `mise run production-deploy`
- application log: `mise run logs-recent`
- AWS certificate: [期限確認と更新](./aws-runtime-auth.md#certificate期限確認)
- Cloudflare package/token: [更新と検証](./cloudflare-tunnel.md)
- OCI変更: [plan review](./oci-network.md#terraform変更時の確認)

構成変更後は各Terraform rootで通常planが`No changes`になること、両systemd serviceが`enabled`かつ`active`であること、外部readinessが成功することを確認します。
