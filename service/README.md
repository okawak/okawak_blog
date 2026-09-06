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

`production-deploy`・`quick-deploy`・`build-deployment`はVPS内部用として通常の`mise tasks ls`から隠しています。初期構築や障害対応で直接操作する場合は、VPSのインストール先で`mise tasks ls --hidden`を参照できます。旧`stop`・`service`・`bin`・`deploy` taskは一括配備へ集約し、旧`status`・`logs`・`logs-recent`・`restart`は管理端末の`*-vps`へ移行しました。

## VPS build tool

production buildはTopcoat CLIを使います。Topcoat CLIのversionは`mise.toml`と`mise.lock`でframeworkと同じ0.7.0へ固定します。

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
git status --short
```

Topcoat CLIが0.7.0で、`mise run check-deps`が成功し、Git差分が空であれば正常です。`mise run build-project`とproduction用のstaged buildはTopcoatのstandalone Tailwind integrationを使い、Bun package installへ依存しません。

`mise run production-deploy`は稼働中のasset directoryを直接buildしません。`target/assets-staged`にhash付きCSS / JavaScript / faviconを揃え、service停止後に`bin/okawak_blog`と、Topcoatがbinaryの隣から読む`bin/assets`を同じreleaseへ切り替えます。stagingはWebAssemblyを拒否します。起動後のhealth / readinessが失敗した場合は旧binaryと旧assetsを復元し、調査用の失敗bundleを`bin/assets.failed`へ残します。

配備時はsystemd unitの`WorkingDirectory`をVPSのrepository root、`ExecStart`を配備したbinaryの絶対パスへ置き換えてインストールします。Git管理下のunit fileは変更しません。`ProtectHome=true`は維持するため、インストール先には`/opt`や`/srv`など、serviceから読める場所を使います。

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

公開経路の運用、hostname、更新、障害対応は[Cloudflare Tunnel runbook](../docs/operations/cloudflare-tunnel.md)に従います。Tunnel、Published application、DNSはCloudflare Dashboardで管理し、Cloudflare resourceをTerraformへimportしません。

repositoryの`cloudflared.service`はremote-managed Tunnelを次の境界で起動します。

- originは`http://127.0.0.1:8008`
- Tunnel tokenは`/etc/cloudflared/token`から読む
- tokenをunit、environment、`mise.toml`、Git管理下のfileへ埋め込まない
- package管理版を使うため`--no-autoupdate`を指定する
- `okawak_blog.service`との依存は`Wants`に留め、application restart中もTunnel processを維持する

Oracle Linux 9ではCloudflare公式RPM repositoryを使用します。repository経由にすることで、以後は`dnf upgrade cloudflared`で更新できます。

```bash
curl -fsSL \
  https://pkg.cloudflare.com/cloudflared-ascii.repo |
  sudo tee /etc/yum.repos.d/cloudflared.repo

sudo dnf install -y cloudflared
```

VPSへunitを配置する前に、RPMが導入するbinary pathが`/usr/local/bin/cloudflared`であることと、`cloudflared --version`がtoken fileをsupportする`2025.4.0`以上であることを確認します。

```bash
command -v cloudflared
cloudflared --version
```

専用userとtoken directoryを作ります。

```bash
getent passwd cloudflared || sudo useradd \
  --system \
  --home-dir /var/lib/cloudflared \
  --shell /sbin/nologin \
  cloudflared

sudo install \
  -d \
  -o root \
  -g cloudflared \
  -m 0750 \
  /etc/cloudflared
```

Dashboardから取得したtokenはshell argumentへ入れず、対話入力で配置します。tokenの値やfile内容を出力しません。

```bash
sudo bash -c '
umask 027
read -rsp "Tunnel token: " token
printf "\n"
printf "%s" "$token" > /etc/cloudflared/token
'
sudo chown root:cloudflared /etc/cloudflared/token
sudo chmod 0640 /etc/cloudflared/token
```

unitを配置して起動します。

```bash
sudo install \
  -o root \
  -g root \
  -m 0644 \
  service/cloudflared.service \
  /etc/systemd/system/cloudflared.service

sudo systemctl daemon-reload
sudo systemctl enable --now cloudflared
```

```bash
sudo systemctl is-enabled cloudflared
sudo systemctl is-active cloudflared
sudo systemctl status cloudflared --no-pager
sudo journalctl -u cloudflared --since '10 minutes ago' --no-pager
```

本番hostnameは`okawak.net`と`www.okawak.net`です。どちらもCloudflare Tunnelへ接続し、OCIの80/443 ingressと直接公開用reverse proxyは使用しません。SSHは60022でLISTENし、22は新規VPSのbootstrap用ingressとしてのみ維持します。

## テスト

`mise run test-service`で運用まわりのテストをまとめて実行します。systemd unitの設定テストは`service/tests/`、デプロイ・証明書更新スクリプトのテストは`scripts/tests/`に置きます。
