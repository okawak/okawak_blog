# Operations

本番VPSとAWS境界の運用手順をまとめます。恒久的な責務と依存方向は[architecture.md](../architecture/architecture.md)を一次情報とし、このdirectoryには移行、検証、rollbackの手順を置きます。

## Runbooks

- [AWS runtime認証のIAM Roles Anywhere移行](./aws-runtime-auth-migration.md)
  - 現行Secrets Manager rotationの停止
  - IAM Roles AnywhereのAWS側準備
  - VPS、Rust AWS SDK、systemdの切替
  - 検証、rollback、旧credential撤去
- [VPS runtime service](../../service/README.md)
  - 現行systemd service
  - static credential反映timer
  - health / readinessとartifact cache

## Terraformの扱い

`terraform/`は参照専用です。このrepositoryからTerraform commandを実行せず、fileも変更しません。AWS Consoleまたは権限を持つ管理端末で行った変更はTerraform stateへ反映されないため、移行中および移行後にこのrepositoryのTerraformを`apply`しないでください。特に現行のSecrets Manager rotation、IAM user、access keyは再作成・再有効化される可能性があります。

AWS側の管理方法を将来変更する場合は、Terraform read-only方針自体を先に見直し、別Issueでstate移行とimport / removeの計画を立てます。
