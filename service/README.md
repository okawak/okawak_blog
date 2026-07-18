# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

AWS側のrotation停止、IAM Roles Anywhereのresource準備、VPS切替、rollback、旧key撤去は[docs/operations/aws-runtime-auth-migration.md](../docs/operations/aws-runtime-auth-migration.md)を一次手順とします。

## AWS credentials

production serviceはIAM Roles Anywhereの`credential_process`を使います。

```text
/usr/local/bin/aws_signing_helper
/etc/okawak_blog/aws/config
/etc/okawak_blog/aws/client-cert.pem
/etc/okawak_blog/aws/client-key.pem
```

systemd unitは次を明示します。

```text
AWS_PROFILE=blog-s3
AWS_CONFIG_FILE=/etc/okawak_blog/aws/config
AWS_EC2_METADATA_DISABLED=true
```

`ProtectHome=true`を維持するため、serviceは`~/.aws`へ依存しません。AWS SDKはhelperから期限付きrole credentialを取得し、期限前に再取得します。temporary credentialをfileへ書くtimerやapplication独自のrefresh処理は導入しません。

helper、certificate、private key、AWS configの作成・配置・単体検証は移行runbookの[Phase 2](../docs/operations/aws-runtime-auth-migration.md#phase-2-管理端末からvpsへcredential-helperとcertificateを配置)に従います。

## Rollback用static credential

移行中に作成した次のfileは、IAM Roles Anywhereの安定観測が終わるまでrollback用として維持します。

```text
/var/lib/okawak_blog/aws/credentials
```

production unitはこのfileを参照しません。rollback時だけunitを旧設定へ戻し、次を復元します。

```text
Environment=AWS_SHARED_CREDENTIALS_FILE=/var/lib/okawak_blog/aws/credentials
```

`credentials-bootstrap`は、home配下の長期`blog-s3` profileをruntime fileへ一度だけ複製する移行・rollback専用taskです。credential値を標準出力へ表示せず、既存fileの内容が異なる場合は上書きを拒否します。

```bash
mise run credentials-bootstrap
```

bootstrap元profileは`mise.toml`の`OKAWAK_BLOG_BOOTSTRAP_SOURCE_PROFILE=blog-s3`を既定値とします。これは手動taskだけの設定であり、systemd serviceへは渡しません。別の検証用pathを使う場合だけ、次のenvを指定できます。

```bash
OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE=/tmp/okawak-blog-runtime/aws/credentials \
  ./service/bootstrap_aws_credentials.sh
```

override先にもruntime専用directoryを指定してください。スクリプトは既存directoryのmodeやownerを変更せず、既存directoryが実行userの所有で書き込み可能な場合だけ利用します。`/tmp/credentials`のように共有directoryを直接親にするpathは拒否します。rollbackの全手順は移行runbookの[Rollback](../docs/operations/aws-runtime-auth-migration.md#rollback)を参照してください。

## Runtime probes

```bash
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

- `/api/health`: process liveness。artifactの状態は確認しません。
- `/api/ready`: configured `ArtifactReader`からsite metadataを読めた場合だけ`200 OK`を返します。直前のimmutable releaseでcache済みmetadataを配信できる場合も`200 OK`です。利用可能なsnapshotがない初回起動時やmetadataを読めない場合は`503 Service Unavailable`です。

## Artifact cache

本番のS3 readerは、release snapshotとそのimmutable artifactをprocess memoryでcacheします。

- `OKAWAK_BLOG_ARTIFACT_CACHE_TTL_SECONDS=5`: production unitの既定値
- TTL内は同じrelease snapshotを再利用するため、新しい`current.json`の反映には最大でTTL分の遅延が生じる
- TTL経過時にrelease identityが同じなら、取得済みartifactは引き続きcacheする
- snapshot更新に失敗した場合は、直前のimmutable releaseを期限なく配信し、次のTTLで更新を再確認する
- 運用中に`current.json`が消えた場合もlegacy rootへ戻さず、直前のimmutable releaseを維持する
- `0`を指定するとsnapshotとartifactのcacheを無効化する
- 初回起動時、legacy root、TTLが`0`の場合はstale fallbackしない
- artifactは必要時にcacheするため、未取得objectのS3 readまで失敗したrequestにはfallbackしない
- load errorはcacheしない
- local readerにはcacheを適用しない

値は0以上の整数秒で指定します。不正値の場合はserver起動時のconfiguration errorになります。
