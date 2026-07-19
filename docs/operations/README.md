# Operations

本番VPSとcloud境界の運用手順をまとめます。恒久的な責務と依存方向は[architecture.md](../architecture/architecture.md)を一次情報とし、このdirectoryには検証、更新、障害対応の手順を置きます。

## Runbooks

- [本番環境の初期構築](./production-setup.md)
  - 管理端末、AWS、OCI、VPS、Cloudflare Dashboardをまたぐ構築順序
  - secretの配置境界と構築完了時の確認
- [AWS runtime認証](./aws-runtime-auth.md)
  - CA、IAM Roles Anywhere、`credential_process`の初期構築
  - caller identity、S3、serviceの検証
  - X.509 certificateの期限確認と更新
- [AWS Terraform](./aws-terraform.md)
  - state backendの初回bootstrap
  - AWS resourceのplan、apply、GitHub Actions設定
- [OCI network](./oci-network.md)
  - 管理端末からのTerraform初期構築
  - Reserved Public IP、SSH、Tunnel egress、Terraform planの確認
- [Cloudflare Tunnel](./cloudflare-tunnel.md)
  - DashboardとVPSでのTunnel初期構築
  - package、token、systemd service、hostname、更新、障害対応
- [VPS runtime service](../../service/README.md)
  - 現行systemd service
  - IAM Roles Anywhereのruntime設定
  - health / readinessとartifact cache

## Terraformの扱い

Codexを含む通常のrepository作業では`terraform/`をread-onlyとし、Terraform commandも実行しません。repository ownerがinfra変更を行う場合は、stateとplanを共有せず、想定外のdestroyまたはreplaceがないことをresource address単位でreviewします。
