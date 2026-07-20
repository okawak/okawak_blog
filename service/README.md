# Runtime service

本番のLeptos SSR serverは`okawak_blog.service`で起動し、S3 artifact readerを使います。

本番環境の構成順序は[本番環境の初期構築](../docs/operations/production-setup.md)、IAM Roles Anywhereの検証、certificate更新、障害切り分けは[AWS runtime認証](../docs/operations/aws-runtime-auth.md)を一次手順とします。

## VPS build tool override

Oracle Linux 9のglibcでは、miseのGitHub backendが配布する`cargo-leptos 0.3.7`を実行できません。本番VPSだけは同じversionをsourceからbuildし、repository固有のlocal configでmise配布版を無効化します。CIと開発端末は`mise.toml`と`mise.lock`のGNU版を引き続き使用します。

VPSの運用userで次を実行します。

```bash
cargo install --locked cargo-leptos --version 0.3.7
cd /opt/okawak_blog
install -m 0644 mise.local.toml.example mise.local.toml
mise settings set locked true
```

`mise.local.toml`はGit管理外です。`[settings].disable_tools`はmise配布版の`cargo-leptos`だけを無効化します。`mise settings set locked true`は運用userのglobal settingへ保存され、tracked `mise.lock`以外の解決を継続的に禁止します。lockfileにmusl用entryが含まれていても、musl版を選択する設定ではありません。

新しいSSH sessionで設定と選択binaryを確認します。

```bash
cd /opt/okawak_blog
mise settings get disable_tools
mise settings get locked
command -v cargo-leptos
cargo leptos --version
mise run check-deps
git status --short
```

`command -v cargo-leptos`がmiseのinstall directoryではなく運用userのCargo bin directoryを示し、versionが`0.3.7`、Git差分が空であれば正常です。`mise run build-project`とproduction用のstaged buildは`web-install`にも依存するため、fresh checkoutでもBun依存を個別に導入する必要はありません。

`mise run production-deploy`は稼働中の`target/site`を直接buildしません。`target/site-staged`にhash付きCSS / JavaScript / WebAssemblyを揃え、service停止後にsite、binary、binaryと同じdirectoryでLeptosが読む`bin/hash.txt`を同じreleaseへ切り替えます。起動後のhealth / readinessが失敗した場合は旧releaseを復元し、調査用の失敗siteを`target/site-failed`へ残します。

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
