# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

AWS側のrotation停止、IAM Roles Anywhereのresource準備、VPS切替、rollback、旧key撤去は[docs/operations/aws-runtime-auth-migration.md](../docs/operations/aws-runtime-auth-migration.md)を一次手順とします。この文書のstatic credential fileは移行中だけのrollback手段です。

## AWS credentials

runtime専用の共有credentials fileは次の場所へ置きます。

```text
/var/lib/okawak_blog/aws/credentials
```

このdirectoryのrootはsystemdの`StateDirectory=okawak_blog`が作成し、unitは`AWS_SHARED_CREDENTIALS_FILE`でfileを明示します。`ProtectHome=true`を維持するため、serviceは`~/.aws/credentials`を読みません。

`credentials-bootstrap`は、現在S3読取に成功しているhome配下の`blog-s3` profileを、上記runtime fileへ一度だけ複製します。AWS CLIのcredential resolutionを使い、credential値を標準出力へ表示しません。既存runtime fileの内容が異なる場合は上書きを拒否します。

このbootstrapはcredentialの取得・rotation・定期更新を行いません。S3 readerへSecrets Manager権限や管理権限を追加せず、IAM Roles Anywhereへ切り替えるまで現在のkeyをrollback用に維持します。現行のrotation Lambdaは手動実行しないでください。

前提:

- `okawak` userの`blog-s3` profileでS3読取に成功する
- AWS CLIが導入済み
- `okawak` userがruntime directoryを作成できる。必要な場合だけ、scriptがそのdirectoryの作成に`sudo`を使う

bootstrap前にidentityとS3読取を確認します。出力されるARNだけを比較し、credential値は表示・記録しないでください。

```bash
AWS_PROFILE=blog-s3 aws sts get-caller-identity
AWS_PROFILE=blog-s3 aws s3api head-object \
  --bucket okawak-blog-resources-bucket \
  --key current.json
```

Secrets Managerのmetadata確認はVPSではなく、管理端末のadmin identityで行います。`oci-blog-reader`の`DescribeSecret`が`AccessDenied`になる場合、その権限を追加しません。

### 2025年版cronからの移行

home配下のcredentialsから移行する初回だけ、serviceのdeploy前にruntime credentialsを配置します。

```bash
mise run credentials-bootstrap
mise run production-deploy
curl --fail http://127.0.0.1:8008/api/ready
```

新serviceのreadiness、home、実記事を確認してから`crontab -e`で次の旧entryだけを削除します。`crontab -r`は他のentryも削除するため使いません。repositoryから新しいcredential refresh timerは導入しません。

```text
5 4 * * * /usr/local/bin/update_aws_creds.sh
```

bootstrap元profileは`mise.toml`の`OKAWAK_BLOG_BOOTSTRAP_SOURCE_PROFILE=blog-s3`を既定値とします。これは手動taskだけの設定であり、systemd serviceへは渡しません。別の検証用pathを使う場合だけ、次のenvを指定できます。

```bash
OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE=/tmp/okawak-blog-runtime/aws/credentials \
  ./service/bootstrap_aws_credentials.sh
```

override先にもruntime専用directoryを指定してください。スクリプトは既存directoryのmodeやownerを変更せず、既存directoryが実行userの所有で書き込み可能な場合だけ利用します。`/tmp/credentials`のように共有directoryを直接親にするpathは拒否します。

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
