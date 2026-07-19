# Cloudflare Tunnel

## 現行構成

VPSのLeptos SSR serverは`127.0.0.1:8008`で待ち受け、`cloudflared`が外向きTunnel経由でCloudflareへ接続します。VPSの80/443はInternetへ公開しません。

```text
Browser
  -> Cloudflare (TLS / DNS / proxy)
  -> Cloudflare Tunnel
  -> cloudflared.service
  -> http://127.0.0.1:8008
  -> okawak_blog.service
```

管理境界は次の通りです。

- Cloudflare Dashboard: Tunnel、Published application、DNS
- OCI Terraform: Reserved Public IP、security list、VPS resource
- repository: `cloudflared.service`、application service、運用手順

Cloudflare resourceはTerraformへimportしません。変更時はDashboardの設定とこの文書の現行構成を揃えます。

## 初期構築の順序

### 1. Cloudflare Dashboard

対象zoneがCloudflareで`Active`になり、管理端末からDashboardを操作できることを確認します。

`Networking` > `Tunnels`でremote-managed Tunnelを作成します。

- Tunnel名: `okawak-blog-vps`
- connector: Cloudflared

Dashboardが表示するinstall commandからtokenを取得します。commandやtokenをGit、Issue、PR、チャット、shell historyへ貼り付けません。この時点ではproduction DNSを変更しません。

### 2. VPS

後述の手順でRPM、専用user、token file、repository管理のsystemd unitを配置します。次を満たすことを確認します。

```bash
sudo systemctl is-enabled cloudflared
sudo systemctl is-active cloudflared
sudo journalctl -u cloudflared --since '10 minutes ago' --no-pager
```

DashboardでTunnelとconnectorが`Healthy`になってからhostnameを追加します。

### 3. Cloudflare Dashboard

Published applicationを一件ずつ追加します。

- `okawak.net` -> `http://127.0.0.1:8008`
- `www.okawak.net` -> `http://127.0.0.1:8008`

既存DNS recordと競合する場合は、対象hostnameだけを確認して置き換えます。両hostnameを同時に削除しません。保存後、DNS target、Tunnel health、外部readinessを一件ずつ確認します。

## Public hostname

本番では次の2 hostnameを同じoriginへ接続します。

| Hostname | Service URL | DNS target |
| --- | --- | --- |
| `okawak.net` | `http://127.0.0.1:8008` | `<Tunnel ID>.cfargotunnel.com` |
| `www.okawak.net` | `http://127.0.0.1:8008` | `<Tunnel ID>.cfargotunnel.com` |

DNSはProxied、TTL Autoとします。zone apexのCNAMEはCloudflareのCNAME flatteningによって利用できます。

DashboardのTunnel IDとconnectorのReplica IDは別物です。DNS targetにはTunnel overviewに表示されるTunnel IDを使い、Replica IDを使いません。

## Packageとsystemd service

Oracle LinuxではCloudflare公式RPM repositoryから`cloudflared`を導入します。

```bash
curl -fsSL \
  https://pkg.cloudflare.com/cloudflared-ascii.repo |
  sudo tee /etc/yum.repos.d/cloudflared.repo

sudo dnf install -y cloudflared
command -v cloudflared
cloudflared --version
rpm -q cloudflared
```

RPMのbinary pathは`/usr/local/bin/cloudflared`です。repositoryのunitを配置します。

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

package管理版を使うためunitは`--no-autoupdate`を指定します。更新は明示的に行います。

```bash
sudo dnf upgrade cloudflared
sudo systemctl restart cloudflared
cloudflared --version
sudo systemctl is-active cloudflared
curl --fail https://www.okawak.net/api/ready
```

## Token

remote-managed Tunnel tokenは`/etc/cloudflared/token`だけに保存します。tokenをGit、unit、environment、`mise.toml`、command argument、ログへ残しません。

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

sudo bash -c '
umask 027
read -rsp "Tunnel token: " token
printf "\n"
printf "%s" "$token" > /etc/cloudflared/token
'
sudo chown root:cloudflared /etc/cloudflared/token
sudo chmod 0640 /etc/cloudflared/token
```

tokenをrotateする場合はDashboardで新しいtokenを取得し、同じ方法でfileを書き換えて`cloudflared`を再起動します。terminalへtokenを表示しません。

## 検証

VPSでserviceとoriginを確認します。

```bash
sudo systemctl is-enabled cloudflared
sudo systemctl is-active cloudflared okawak_blog
sudo systemctl status cloudflared --no-pager
sudo journalctl -u cloudflared --since '30 minutes ago' --no-pager

curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

管理端末から公開経路を確認します。

```bash
curl --fail --output /dev/null \
  --write-out 'apex %{http_code}\n' \
  https://okawak.net/

curl --fail --output /dev/null \
  --write-out 'www %{http_code}\n' \
  https://www.okawak.net/

curl --fail https://www.okawak.net/api/ready
```

DashboardではTunnelが`Healthy`で、connectorが接続済みであることを確認します。ブラウザではhome、category、article、戻る・進む操作を確認し、WASM初期化errorがないことも確認します。

## 障害対応

1. `okawak_blog`のhealth/readinessをlocalhostで確認する
2. `cloudflared`の状態とjournalを確認する
3. DashboardのTunnel health、connector、Published applicationを確認する
4. DNS targetがTunnel IDと一致することを確認する
5. package更新直後なら直前のRPMへ戻すか、原因を確認してserviceを再起動する

originの80/443と直接公開用reverse proxyは閉鎖済みです。直接公開を復元する場合は、先にreverse proxy、TLS、OCI ingressを安全に構成してからDNSを変更します。DNSだけをReserved Public IPへ向けても公開できません。

SSHはTunnelへ移さず、日常運用と保守作業のためにReserved Public IPのTCP 60022を使います。OCI security listでは新規VPSのbootstrap用にTCP 22も許可しますが、通常時のsshdは60022だけでLISTENします。

## 公式資料

- [Cloudflare Tunnel](https://developers.cloudflare.com/tunnel/)
- [Tunnel run parameters](https://developers.cloudflare.com/tunnel/advanced/run-parameters/)
- [Update cloudflared](https://developers.cloudflare.com/tunnel/downloads/update-cloudflared/)
