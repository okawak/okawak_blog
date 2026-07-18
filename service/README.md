# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

## AWS credentials

runtime専用の共有credentials fileは次の場所へ置きます。

```text
/var/lib/okawak_blog/aws/credentials
```

このdirectoryのrootはsystemdの`StateDirectory=okawak_blog`が作成し、unitは`AWS_SHARED_CREDENTIALS_FILE`でfileを明示します。`ProtectHome=true`を維持するため、serviceは`~/.aws/credentials`を読みません。

credential更新スクリプトは、`okawak` userの通常環境にある`secret-get` profileでAWS Secrets Managerを読みます。現在のruntime credentialと同じならfileとserviceに触れず、変更された場合だけatomicに置き換えてserviceを再起動します。再起動後は`/api/ready`を確認し、失敗時は旧credentialを戻して再起動を試みた上で、systemd serviceの失敗として記録します。

この処理はSecrets Managerに保存済みのcredentialをruntimeへ反映するもので、IAM access keyのrotation自体ではありません。`secret-get`と`blog-s3`が同じIAM userである場合、そのuserの旧keyを削除してから新keyを取得する自己rotationは成立しません。AWS側の安全なrotation基盤が整うまで、現行のrotation Lambdaを手動実行しないでください。

前提:

- `okawak` userが`secret-get` profileを利用できる
- AWS CLIと`jq`が導入済み
- `okawak` userがruntime directory作成とservice再起動に必要な`sudo`権限を持つ
- `/usr/bin/systemctl restart okawak_blog`だけをpasswordなしで実行できるよう、`sudoers`を最小権限で設定する

`secret-get`のbootstrap identityを確認します。出力されるARNだけを比較し、credential値は表示・記録しないでください。

```bash
AWS_PROFILE=secret-get aws sts get-caller-identity
AWS_PROFILE=blog-s3 aws sts get-caller-identity
```

両者が同じIAM userなら、現行キーを削除するrotationは無効のまま維持します。長期的にはIAM Roles Anywhereと`credential_process`など、長期AWS access keyをbootstrapに使わない方式へ移行します。

service再起動用の`sudoers`は`visudo`で作成します。`systemctl`のpathはVPS上の`command -v systemctl`で確認してください。

```text
okawak ALL=(root) NOPASSWD: /usr/bin/systemctl restart okawak_blog
```

### 2025年版cronからの移行

home配下のcredentialsから移行する初回だけ、serviceのdeploy前にruntime credentialsを配置します。

```bash
mise run credentials-bootstrap
mise run production-deploy
curl --fail http://127.0.0.1:8008/api/ready
mise run credentials-refresh-install
mise run credentials-refresh
```

新timerの動作を確認してから`crontab -e`で次の旧entryだけを削除します。`crontab -r`は他のentryも削除するため使いません。

```text
5 4 * * * /usr/local/bin/update_aws_creds.sh
```

timerは毎日4:05に実行し、VPSが停止していた場合は`Persistent=true`により次回起動後に補完します。状態とjournalはrootの`mise` taskで確認できます。

```bash
mise run credentials-refresh-status
mise run credentials-refresh-logs
```

`runtime credentials unchanged`は正常です。この場合はfile更新もservice再起動も行いません。`runtime credentials updated`の場合だけ、service再起動とreadiness確認が行われます。

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
