# Operations

本番VPSとcloud境界の運用手順をまとめます。恒久的な責務と依存方向は[architecture.md](../architecture/architecture.md)を一次情報とし、このdirectoryには検証、更新、障害対応の手順を置きます。

## Runbooks

- [AWS runtime認証](./aws-runtime-auth.md)
  - IAM Roles Anywhereと`credential_process`の現行構成
  - caller identity、S3、serviceの検証
  - X.509 certificateの期限確認と更新
- [OCI Public IP固定化とTerraform変更計画](./oci-network-terraform-plan.md)
  - 既存OCI resourceとstateの照合
  - Reserved Public IPの3段階apply
  - Cloudflare DNS、reboot、Tunnel移行との境界
- [VPS runtime service](../../service/README.md)
  - 現行systemd service
  - IAM Roles Anywhereのruntime設定
  - health / readinessとartifact cache

## Terraformの扱い

Codexを含む通常のrepository作業では`terraform/`をread-onlyとし、Terraform commandも実行しません。repository ownerがinfra変更を行う場合は、stateとplanを共有せず、想定外のdestroyまたはreplaceがないことをresource address単位でreviewします。
