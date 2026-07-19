# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

IAM Roles Anywhereの検証、certificate更新、障害切り分けは[docs/operations/aws-runtime-auth.md](../docs/operations/aws-runtime-auth.md)を一次手順とします。

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

helper、certificate、private key、AWS configの配置と検証はruntime認証runbookに従います。productionでは`AWS_SHARED_CREDENTIALS_FILE`を指定せず、`/var/lib/okawak_blog/aws/credentials`やhome配下のlong-lived access keyへfallbackしません。

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
