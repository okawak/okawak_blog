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
OKAWAK_BLOG_RUNTIME_CREDENTIAL_FILE=/tmp/okawak-blog-credentials \
  ./service/update_aws_creds.sh
```

## Runtime probes

```bash
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

- `/api/health`: process liveness。artifactの状態は確認しません。
- `/api/ready`: configured `ArtifactReader`からsite metadataを読めた場合だけ`200 OK`を返します。credentials、S3、local artifact、JSON decodeの問題がある場合は`503 Service Unavailable`です。
