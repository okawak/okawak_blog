# Runtime service

本番のTopcoat SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

本番環境の構成順序は[本番環境の初期構築](../docs/operations/production-setup.md)、IAM Roles Anywhereの検証、certificate更新、障害切り分けは[AWS runtime認証](../docs/operations/aws-runtime-auth.md)を一次手順とします。

## 操作する端末と設定

通常の運用コマンドは管理端末のrepository rootで実行します。VPSへの接続はSSH configを使い、必要な`sudo` passwordはSSHの対話端末で入力します。

| 管理端末のコマンド | VPSで行う処理 |
| --- | --- |
| `mise run deploy-vps` | 最新mainの取得、ビルド、配備、health/readiness確認 |
| `mise run status-vps` | service状態表示 |
| `mise run logs-recent-vps` | 直近50行のjournal表示 |
| `mise run logs-vps` | journalの追尾（Ctrl-Cで終了） |
| `mise run restart-vps` | application serviceの再起動 |
| `mise run rotate-runtime-certificate` | 管理端末で発行した証明書の配置と検証 |

`mise.local.toml`は管理端末専用です。[設定例](../mise.local.toml.example)にSSH接続先や証明書発行端末名を設定します。VPSにはmise本体とGit管理下の`mise.toml` / `mise.lock`を用意しますが、`mise.local.toml`は不要です。VPSのruntime認証は`/etc/okawak_blog/aws/`、serviceの実行設定はsystemd unitで管理します。

接続先は管理端末の`mise.local.toml`の`[env]`に`OKAWAK_BLOG_VPS_SSH_TARGET`として設定します。taskの引数で一時的に上書きできます。既定の接続先はなく、未設定・空欄なら接続前に停止します。hostname、user、鍵、Portは通常のSSH configで指定します。証明書更新のSSH/SCPも同じ設定に従います。

たとえばSSH configの`Host oci`を使う場合、管理端末の`mise.local.toml`へ次を設定します。接続先は端末ごとの設定なので、共有する`mise.toml`には固定値を置きません。

```toml
[env]
OKAWAK_BLOG_VPS_SSH_TARGET = "oci"
OKAWAK_BLOG_VPS_REPO_DIR = "/opt/okawak_blog"
```

```bash
mise run deploy-vps -- my-vps
mise run deploy-vps -- --help
```

`OKAWAK_BLOG_VPS_REPO_DIR`にはVPS上の既存repository / インストール先を指定します。管理端末の`mise.local.toml`での設定が必須で、未設定・空欄・相対パスは接続前に拒否します。既定のパスへのfallbackはありません。パスには英数字、`/`、`.`、`_`、`-`を使用できます。

`deploy-vps`はVPSのlogin shellを開き、指定したディレクトリで既存の`production-deploy` taskを呼びます。前提は、VPSの運用userでGitHubからpullでき、login shellのPATHにmiseがあり、後述のbuild tool設定が済んでいることです。VPS checkoutがmain以外、未commit差分あり、またはorigin/mainに含まれないcommitを持つ場合は、ビルド・service操作より前に停止します。管理端末の未push・未mergeのソースやbinaryを転送する機能はありません。

実行中は端末とSSH接続を維持し、別のデプロイや証明書更新を並行実行しないでください。VPS側の失敗は管理端末へ非ゼロの終了コードで返し、デプロイ成功時は`production-deploy completed`を表示します。公開URLのreadinessとホーム・記事の表示も確認します。

```bash
curl --fail -H 'Cache-Control: no-cache' \
  "https://www.okawak.net/api/ready?deploy-check=$(date +%s)"
```

`production-deploy`・`quick-deploy`・`build-deployment`はVPS内部用として通常の`mise tasks ls`から隠しています。初期構築や障害対応で直接操作する場合は、VPSのインストール先で`mise tasks ls --hidden`を参照できます。

## VPS build tool

production buildはTopcoat CLIを使います。CLIのversionは`mise.toml`と`mise.lock`、frameworkのversionはworkspaceの`Cargo.toml`を正本とし、同じversionへ揃えます。

VPSの運用userで次を実行します。以下は`/opt/okawak_blog`の例です。別の場所に配置している場合は、`cd`先をそのパスに置き換えます。

```bash
cd /opt/okawak_blog
mise settings set locked true
```

`mise settings set locked true`は運用userのglobal settingへ保存され、tracked `mise.lock`以外の解決を継続的に禁止します。

新しいSSH sessionで設定と選択binaryを確認します。

```bash
cd /opt/okawak_blog
mise settings get locked
topcoat fmt --version
mise run check-deps
mise run versions-check
git status --short
```

Topcoat CLIがworkspaceのTopcoat frameworkと同じversionで、`mise run check-deps`と`mise run versions-check`が成功し、Git差分が空であれば正常です。`mise run build-project`とproduction用のstaged buildはTopcoatのstandalone Tailwind integrationを使い、Bun package installへ依存しません。

`mise run production-deploy`は稼働中のasset directoryを直接buildしません。`target/assets-staged`にhash付きCSS / JavaScript / faviconを揃え、service停止後に`bin/okawak_blog`と、Topcoatがbinaryの隣から読む`bin/assets`を同じreleaseへ切り替えます。stagingはWebAssemblyを拒否します。起動後のhealth / readinessが失敗した場合は旧binary・旧assets・旧systemd unitを復元し、調査用の失敗bundleを`bin/assets.failed`へ残します。

`bin/assets.failed`が存在する間は次の配備を開始しません。失敗原因の調査が終わり、bundleが不要になったことを確認してからVPSのrepository rootで`rm -rf bin/assets.failed`を実行し、管理端末から`mise run deploy-vps`を再実行します。

配備時はsystemd unitの`WorkingDirectory`をVPSのrepository root、`ExecStart`を配備したbinaryの絶対パスへ置き換えてインストールします。上書き前のunitは同じsystemd directoryの`okawak_blog.service.rollback`へ退避し、失敗時は`daemon-reload`と再起動の前に復元します。初回配備で旧unitがなければ新unitを削除します。unit復元に失敗した場合はbackupを残し、再起動を止めて手動復旧を案内します。Git管理下のunit fileは変更しません。`ProtectHome=true`は維持するため、インストール先には`/opt`や`/srv`など、serviceから読める場所を使います。

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

helper、certificate、private key、AWS configの配置と検証はruntime認証runbookに従います。定期的なclient certificate更新は管理端末から`mise run rotate-runtime-certificate`を実行します。productionでは`AWS_SHARED_CREDENTIALS_FILE`を指定せず、`/var/lib/okawak_blog/aws/credentials`やhome配下のlong-lived access keyへfallbackしません。

## Runtime probes

```bash
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

- `/api/health`: process liveness。artifactの状態は確認しません。
- `/api/ready`: configured `ArtifactReader`からsite metadataを読めた場合だけ`200 OK`を返します。直前のimmutable releaseでcache済みmetadataを配信できる場合も`200 OK`です。利用可能なsnapshotがない初回起動時やmetadataを読めない場合は`503 Service Unavailable`です。

## Runtime logging

`crates/server`の起動情報、readiness failure、page document読取失敗は`tracing` eventとして標準出力へ記録し、systemd journalから確認します。log filterは`RUST_LOG`で指定し、未指定または有効なdirectiveがない場合は`info`を使います。本番unitは`RUST_LOG=info`を明示します。`debug`のようなlevelに加えて、`server=debug,topcoat=warn`のようなtarget別filterも指定できます。不正なdirectiveは無視します。`RUST_LOG`が制御するのは`tracing` eventであり、`crates/infra`に残る既存の標準エラー出力はこの設定の対象外です。

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

## Cloudflare Tunnel

導入、更新、token配置、Dashboard設定、検証、障害対応は[Cloudflare Tunnel runbook](../docs/operations/cloudflare-tunnel.md)を正本とします。Tunnel、Published application、DNSはCloudflare Dashboardで管理し、Cloudflare resourceをTerraformへimportしません。

repositoryの`cloudflared.service`はremote-managed Tunnelを次の境界で起動します。

- originは`http://127.0.0.1:8008`
- Tunnel tokenは`/etc/cloudflared/token`から読む
- tokenをunit、environment、`mise.toml`、Git管理下のfileへ埋め込まない
- package管理版を使うため`--no-autoupdate`を指定する
- `okawak_blog.service`との依存は`Wants`に留め、application restart中もTunnel processを維持する
- 公開経路にOCIの80/443 ingressと直接公開用reverse proxyを使わない

## テスト

`mise run test-service`で運用まわりのテストをまとめて実行します。systemd unitの設定テストは`service/tests/`、submodule同期・デプロイ・証明書更新スクリプトのテストは`scripts/tests/`に置きます。
