# Cloudflare Tunnel移行手順

Issue [#118](https://github.com/okawak/okawak_blog/issues/118)で、OCI VPSのWeb公開をPublic IP経由からCloudflare Tunnelへ移行します。この文書は移行中だけ使うチェックポイント式runbookです。移行完了後は恒久的な運用手順へ必要事項を移し、この文書を削除します。

## 管理境界

- Cloudflare Tunnel、Published application、DNSはCloudflare Dashboardで管理する
- OCI Reserved Public IP、security list、routeは`terraform/oci/`で管理する
- SSHは当面Reserved Public IPを使用する
- `cloudflared`のsystemd unitと非secretの運用手順はrepositoryで管理する
- Tunnel tokenはVPSの`/etc/cloudflared/token`だけに置き、Git、Issue、PR、チャット、`mise.toml`へ記録しない

Cloudflare resourceはTerraformへimportせず、Cloudflare providerも追加しません。

## 進行ルール

1. 一度に一つのStepだけ実行する
2. 各Stepの確認結果をreviewしてから次へ進む
3. `停止条件`に該当した場合は、その場で止めてrollbackまたは原因調査を行う
4. Terraform state、保存済みplan、Tunnel token、秘密鍵を共有しない
5. Tunnelが安定するまでOCIの80/443 ingressと既存DNSを維持する

## Step 1: 移行前baselineを記録する

### 管理端末

```bash
dig okawak.net
dig www.okawak.net

curl --fail --show-error \
  --output /dev/null \
  --write-out 'apex HTTPS %{http_code}\n' \
  https://okawak.net/

curl --fail --show-error \
  --output /dev/null \
  --write-out 'www HTTPS %{http_code}\n' \
  https://www.okawak.net/
```

Cloudflare DashboardのDNS画面で、`okawak.net`と`www`について次をローカルの作業メモへ記録します。

- record type
- content
- Proxy status
- TTL
- rollback時に戻すReserved Public IP

### VPS

```bash
sudo systemctl is-active okawak_blog
sudo systemctl is-active nginx
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
sudo ss -lntp | grep -E ':(80|443|8008)\b'
```

### 次に進む条件

- `okawak_blog`とnginxが`active`
- healthが`OK`
- readinessが`READY`
- apexと`www`がHTTPS 200
- 現在のDNS設定とReserved Public IPを復元できる形で記録済み

### 停止条件

- 現行サイト、health、readinessのいずれかが失敗する
- DNSのrollback先が特定できない

## Step 2: OCIでTunnel用UDP egressを許可する

Cloudflare TunnelはCloudflare edgeへのoutbound TCP/UDP 7844を使用します。現在のTCP egressは維持し、repository ownerが`terraform/oci/network/main.tf`の`oci_core_security_list`へUDP/7844 egressを追加します。Codexは`terraform/`を編集せず、共有されたplan概要だけをreviewします。

このStepでは80/443 ingressを削除しません。

```hcl
egress_security_rules {
  protocol    = 17
  destination = "0.0.0.0/0"
  stateless   = false

  udp_options {
    min = 7844
    max = 7844
  }
}
```

```bash
cd terraform/oci
terraform fmt -check -recursive
terraform validate
terraform plan -out=plan_cloudflare_egress
```

### 次に進む条件

- planは原則`0 to add, 1 to change, 0 to destroy`
- update対象は`oci_core_security_list`だけ
- 差分はUDP/7844 egress追加だけ
- apply後の通常planが`No changes`

### 停止条件

- compute instance、VNIC、subnet、route table、Reserved Public IPのreplaceまたはdestroy
- 80/443 ingressの削除
- 意図しないsecurity ruleの削除

## Step 3: repository側の導入準備を行う

VPS操作前に実装PRを作り、次を追加または更新します。

- repository管理の`cloudflared.service`
- systemd unitの静的test
- `service/README.md`の導入、更新、rollback手順
- `docs/operations/README.md`からの導線
- CloudflareをDashboard管理とする管理境界

unitにはtokenを埋め込まず、`--token-file /etc/cloudflared/token`を使用します。package管理版を使うため`--no-autoupdate`を指定します。

### 次に進む条件

- formatとsystemd unit testが成功する
- unitやtracked fileにtokenが含まれない
- PRがmainへmerge済み

### 停止条件

- unitのcommand lineやenvへtokenを直接記述している
- repositoryから存在しないsecret fileを生成しようとしている

## Step 4: Dashboardでremote-managed Tunnelを作る

Cloudflare Dashboardの`Networking` > `Tunnels`から`cloudflared` Tunnelを作成します。Tunnel名は`okawak-blog-vps`を標準とします。

表示されたinstall commandからTunnel tokenだけを安全な一時メモへ取得します。command全体やtokenをIssue、PR、チャットへ貼り付けません。Dashboardの`service install <TOKEN>`はまだ実行しません。

### 次に進む条件

- Tunnel objectが作成されている
- tokenをVPSへ安全に入力できる
- production hostnameとDNSは未変更

### 停止条件

- tokenがshell historyやtracked fileへ保存された
- 既存DNSが意図せず変更された

## Step 5: VPSへcloudflaredを導入する

Oracle Linux 9 x86-64へCloudflare公式RPMを導入し、versionとbinary pathを確認します。

```bash
sudo dnf install -y \
  https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-x86_64.rpm

command -v cloudflared
cloudflared --version
rpm -q cloudflared
```

専用userとtoken directoryを用意し、tokenは対話入力で`/etc/cloudflared/token`へ配置します。tokenの値は出力しません。

mainへmerge済みのunitを`/etc/systemd/system/cloudflared.service`へinstallし、serviceをenable/startします。

### 次に進む条件

- `cloudflared`が`2025.4.0`以上
- token fileがGit管理外で、必要最小限のuserだけが読める
- `cloudflared.service`が`enabled`かつ`active`
- Dashboard上のTunnelが`Healthy`

### 停止条件

- tokenがjournal、process argument、shell historyへ表示される
- Tunnelが`Down`または`Degraded`のまま安定しない
- `cloudflared`がlocalhostのapplicationへ接続できない

## Step 6: test hostnameでTunnel経路を確認する

Dashboardで`Published application`を追加します。

- hostname: `tunnel-test.okawak.net`
- service: `http://127.0.0.1:8008`

```bash
curl --fail https://tunnel-test.okawak.net/api/health
curl --fail https://tunnel-test.okawak.net/api/ready
curl -I https://tunnel-test.okawak.net/
```

private browser contextでhome、category、article、CSS、画像、WASM、client-side navigation、favicon、console errorを確認します。

### 次に進む条件

- Tunnel経由のhealth、readiness、HTMLが成功する
- `cf-ray`などCloudflare経由を示すheaderがある
- hydrationとclient-side navigationが動く
- WASM初期化errorがない

### 停止条件

- Issue #116のWASM初期化errorが再現する
- static asset、記事、navigationのいずれかが失敗する
- originへの接続errorがjournalへ継続して出る

## Step 7: production hostnameをTunnelへ切り替える

`www.okawak.net`、apex `okawak.net`の順で、一件ずつPublished applicationとDNSをTunnelへ切り替えます。既存recordと競合する場合は、記録済み設定を確認してから対象recordだけを削除し、直ちにTunnel routeを保存します。両方を同時に削除しません。

各hostnameのserviceは`http://127.0.0.1:8008`とします。

### 次に進む条件

- apexと`www`のHTTPS、health、readinessが成功する
- browserでStep 6と同じ確認が成功する
- rollback用の旧DNS設定を維持している

### 停止条件

- DNS record conflictを解消できない
- apexまたは`www`が到達不能になる
- Tunnel経由と旧origin経由のどちらか判別できない

## Step 8: Tunnelを安定確認する

80/443を閉じる前に最低8時間、可能なら24時間確認します。

```bash
sudo systemctl is-active okawak_blog cloudflared
sudo journalctl -u cloudflared --since '8 hours ago' --no-pager
curl --fail https://www.okawak.net/api/ready
```

### 次に進む条件

- Dashboard上でTunnelが継続して`Healthy`
- serviceの異常再起動や継続的なconnection errorがない
- productionの主要pageとbrowser hydrationが正常

### 停止条件

- `Down`または反復する`Degraded`
- applicationまたはTunnelの再起動loop
- productionの表示、asset、WASMに回帰がある

## Step 9: OCIの80/443 ingressを閉じる

repository ownerが`terraform/oci/network/main.tf`からTCP/80とTCP/443のingress ruleだけを削除します。SSH、WireGuard、ICMP、TCP egress、UDP/7844 egressは維持します。

planをreviewし、security listのin-place updateだけであることを確認してからapplyします。apply後、管理端末からReserved Public IPへのHTTP/HTTPSが失敗し、Cloudflare経由のHTTPSが成功することを確認します。

### 次に進む条件

- Public IPへの80/443直接接続が失敗する
- Cloudflare経由のサイト、health、readinessが成功する
- SSHはReserved Public IPで継続できる
- apply後の通常planが`No changes`

### 停止条件

- SSHまたはTunnelまで到達不能になる
- security list以外にreplaceまたはdestroyがある

## Step 10: rebootと恒久文書化を完了する

VPSをrebootし、同じReserved Public IPでSSHへ再接続します。`okawak_blog`と`cloudflared`が自動起動し、Cloudflare経由のサイトが復旧することを確認します。

完了後に次を整理します。

- nginxを通常配信経路から外し、停止またはrollback用途で残す判断
- `production-deploy`のnginx依存
- Cloudflare Dashboard、token rotation、package update、障害復旧の恒久runbook
- `oci-network-terraform-plan.md`とこの移行文書の削除
- Issue #118のclose

## Rollback

### production DNS切替前

test hostnameを削除し、`cloudflared`を停止します。現行のPublic IP経由配信には影響しません。

### production DNS切替後、80/443閉鎖前

apexと`www`を記録済みの旧DNS設定へ戻します。Reserved Public IPへの80/443 ingressはまだ存在するため、旧経路へ戻せます。

### 80/443閉鎖後

Tunnel障害時に旧経路へ戻す場合は、先にOCI Terraformで80/443 ingressを復元し、接続確認後にDNSをReserved Public IPへ戻します。Dashboardだけを先に切り替えません。

## 公式資料

- [Create a tunnel (dashboard)](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel/)
- [Tunnel run parameters](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/)
- [Tunnel with firewall](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/tunnel-with-firewall/)
- [Download cloudflared](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/downloads/)
