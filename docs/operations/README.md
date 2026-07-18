# Operations

本番VPSとAWS境界の運用手順をまとめます。恒久的な責務と依存方向は[architecture.md](../architecture/architecture.md)を一次情報とし、このdirectoryには移行、検証、rollbackの手順を置きます。

## Runbooks

- [AWS runtime認証のIAM Roles Anywhere移行](./aws-runtime-auth-migration.md)
  - 現行Secrets Manager rotationの停止
  - IAM Roles AnywhereのAWS側準備
  - VPS、Rust AWS SDK、systemdの切替
  - 検証、rollback、旧credential撤去
- [AWS runtime認証のTerraform変更計画](./aws-runtime-auth-terraform-plan.md)
  - repository ownerが実装するresourceとfile構成
  - 並行追加とlegacy撤去の2段階apply
  - expected plan、state機密性、rollback
- [VPS runtime service](../../service/README.md)
  - 現行systemd service
  - static credentialの初回bootstrap
  - health / readinessとartifact cache

## Terraformの扱い

Codexを含む通常のrepository作業では`terraform/`をread-onlyとし、Terraform commandも実行しません。一方、認証基盤の最終的なdesired stateはTerraformで管理するのが自然なため、repository ownerが[変更計画](./aws-runtime-auth-terraform-plan.md)に従ってHCLとstateを更新します。

ownerのTerraform変更が準備・reviewされるまでは、現行HCLを`apply`しません。手動でrotationを停止した後に旧HCLをapplyすると、危険なrotationが再有効化される可能性があります。
