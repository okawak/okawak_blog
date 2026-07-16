# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

## AWS credentials

runtime専用の共有credentials fileは次の場所へ置きます。

```text
/var/lib/okawak_blog/aws/credentials
```

このdirectoryのrootはsystemdの`StateDirectory=okawak_blog`が作成し、unitは`AWS_SHARED_CREDENTIALS_FILE`でfileを明示します。`ProtectHome=true`を維持するため、serviceは`~/.aws/credentials`を読みません。

credential更新スクリプトは、`okawak` userの通常環境にある`secret-get` profileでAWS Secrets Managerを読み、runtime専用fileを同一directory内でatomicに置き換えます。

前提:

- `okawak` userが`secret-get` profileを利用できる
- AWS CLIと`jq`が導入済み
- `okawak` userがruntime directory作成とservice再起動に必要な`sudo`権限を持つ

home配下のcredentialsから移行する初回だけ、serviceのdeploy前にruntime credentialsを配置します。

```bash
OKAWAK_BLOG_SKIP_RESTART=1 ./service/update_aws_creds.sh
mise run production-deploy
curl --fail http://127.0.0.1:8008/api/ready
```

以降のcredential rotationでは、スクリプトがfileを置き換えてserviceを再起動します。

```bash
./service/update_aws_creds.sh
```

別の検証用pathへ書く場合だけ、次のenvを指定できます。

```bash
OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE=/tmp/okawak-blog-runtime/aws/credentials \
  ./service/update_aws_creds.sh
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
