[![Publish Content to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

`okawak_blog` は、Obsidian で書いた Markdown を ローカルの`export`が日英の公開Markdownへ抽出し、`publish`とGitHub Actionsが配信用artifactを生成してS3に配置し、それを VPS 上の単一バイナリ Topcoat SSR サーバーと Cloudflare Tunnel で公開する、静的コンテンツ公開基盤 + SSR 表示基盤です。

## 関連文書

- [docs/architecture/architecture.md](./docs/architecture/architecture.md): 現行アーキテクチャと artifact 契約
- [docs/content/obsidian-template.md](./docs/content/obsidian-template.md): Obsidian Markdown のテンプレート
- [mise.local.toml.example](./mise.local.toml.example): 管理端末専用のローカル設定例。[VPS運用](./service/README.md#操作する端末と設定)・[証明書更新](./docs/operations/aws-runtime-auth.md#client-certificate更新)に必要な項目を設定する。VPSには配置しない
- GitHub Issues / PRs: 実装計画、進捗、作業単位の管理

## このリポジトリが担うこと

- 記事は Obsidian で執筆する
- 記事ソースは private な Obsidian リポジトリで管理する
- private原文は必要時にローカルでsubmoduleから取得し、選別済みの日英公開Markdownだけをこのリポジトリへcommitする
- GitHub Actions またはローカル実行の`publish`が公開成果物を生成する
- 生成した HTML / index JSON を S3 に配置する
- Topcoat SSR サーバーが S3 上の成果物を読んで配信する
- VPS + `systemd` + Cloudflare Tunnel で単純に運用できる構成を保つ

## これは何ではないか

このプロジェクトは、一般的なブログ CMS や SaaS ブログサービスを作るものではありません。

現時点での非目標は以下です。

- DB を使った記事管理
- ユーザー認証・認可
- 管理画面
- ブラウザ UI からの記事作成・編集
- マルチユーザー運用
- 複雑なバックオフィス機能

## 目指すアーキテクチャ

### コンテンツパイプライン中心

主役は常駐 API サーバーではなく、公開成果物生成パイプラインです。

1. ローカルの`export`がObsidianの公開対象だけを抽出・正規化する
2. 記事・タグ・UI辞書の必要な項目だけをローカルCodexで翻訳する
3. 訳文を確認・修正し、日英Markdownと辞書をGitで確定する
4. `publish`が公開Markdownから言語別HTML・index JSONを生成する
5. Actionsが同じimmutable S3 releaseへuploadし、公開前に日英を検証する
6. 成功後に`current.json`を切り替える
7. Topcoat SSRが日本語の既存URLと`/en/...`で配信する

抽出と翻訳は`crates/export/`、`publish`側の実装は `crates/publish/` に、公開成果物の読取は `crates/infra/`、配信とUIは `crates/server/` に置きます。`crates/domain/` は両者で共有する契約と純粋ルールを置く場所として扱います。

### ビルド時変換

Markdown から HTML への変換はランタイムではなくビルド時に行います。SSR サーバーは、変換済みの HTML と index データを読み、ルーティング、レイアウト、meta 情報の組み立てに集中します。

### Rust らしい責務分割

- 純粋ロジックと I/O を分離する
- 外部境界は trait で薄く切る
- 型で不正状態を減らす
- 単一バイナリでの運用性を優先する
- ビルド時に解決できる責務をランタイムへ持ち込まない

## 現在の workspace 構成

```text
okawak_blog/
├── crates/
│   ├── domain/               # 公開成果物契約と純粋ルール
│   ├── publish/              # publish CLIと内部module
│   ├── infra/                # artifact reader、source設定、cache
│   └── server/               # Topcoat application、runtime、UI、API、reader composition
├── e2e/                      # 公開サイト全体の browser E2E
├── docs/
│   └── architecture/
├── scripts/                 # 運用スクリプト
│   └── tests/                # スクリプトのテスト
├── service/                 # systemd unitと運用手順
│   └── tests/                # systemd unitのテスト
└── terraform/
```

### 各層の責務

- `crates/domain`: 公開成果物契約、site page contract、純粋関数を中心にした共有ドメイン層
- `crates/export`: private入力の公開対象抽出、参照の正規化、ローカルCodexによる翻訳、手動編集の保護。
- `crates/publish`: Gitの公開Markdownを検証し、言語別URL解決・HTML変換・bookmark enrichment・artifact生成を行う。private vaultやAIは不要。
- `crates/infra`: storage非依存のartifact reader契約と、local / S3実装、source設定、cache。HTTP runtimeやUIには依存しない
- `crates/server`: production `server` binaryを持つ単一のTopcoat application crate。storage非依存のpage load契約、UI / route / metadata、style、reader生成・注入、API、release-aware ETag / Last-Modifiedを構成する
- `e2e`: server / artifact readerをまたぐ、固定artifactベースのbrowser E2E

## 公開成果物のイメージ

```text
site/
├── articles/
│   ├── <category>/
│   │   └── <slug>.html
│   └── index.json
├── categories/
│   ├── <category>.json
│   └── ...
├── pages/
│   ├── about.json
│   └── ...
├── home.json
└── metadata/
    └── site.json
```

`publish`はこれらの成果物を生成し、SSR サーバーはそれらを読んでページを返します。

## データフロー

```text
private Obsidian（ローカルsubmodule）
  -> export / 翻訳・レビュー
  -> 日英Markdown・辞書をGit管理
  -> publish
  -> 言語別HTML / index JSON を生成
  -> AWS S3
  -> Topcoat SSR server
  -> Browser
```

日本語の正本はprivate Obsidianです。`export`が公開対象だけを`content/ja`へ抽出し、英語の`content/en`とタグ辞書を作ります。公開用Markdownと辞書はGit管理し、`publish`とActionsはこれらだけを読みます。Actionsにprivate ObsidianやAIの認証情報は不要です。HTML・index JSON・翻訳候補・cacheはGit対象外です。

## Obsidian Front Matter

`export`が扱う執筆用MarkdownにはYAML front matterが必要で、`is_completed: true`だけを抽出します。publishの入力は[公開Markdown契約](docs/content/public-markdown.md)を使います。役割判定には `kind` を使います。

```yaml
---
title: "Rust Performance Notes"
kind: article
tags: ["rust", "performance"]
summary: "Short summary shown in lists and metadata."
is_completed: true
priority: 1
created: "2025-01-15T10:00:00+09:00"
updated: "2025-01-16T09:30:00+09:00"
category: "tech"
---
```

採用している `kind` は次の 4 種類です。

- `article`: 通常記事。`kind` 省略時の default です。
- `category`: カテゴリ landing page です。
- `page`: 固定ページです。`page: about` のように page key を持ちます。
- `home`: home の intro に差し込む optional fragment です。

主なフィールドの役割は次の通りです。

- `title`: 記事タイトル。必須です。
- `kind`: コンテンツ種別です。省略時は `article` として扱います。
- `tags`: タグ一覧。省略可能です。
- `summary`: 一覧やメタ情報に使う短い説明。省略可能です。
- `is_completed`: 公開対象かどうかを示すフラグ。`true` の記事だけを出力します。
- `priority`: 並び順や強調表示に使う優先度。省略可能です。
- `created`: 作成日時。必須です。
- `updated`: 更新日時。必須です。
- `category`: `article` と `category` で使うカテゴリキーです。
- `page`: `kind: page` のときに使う固定ページキーです。

本文は closing `---` の次の行から始まり、Obsidian link や bookmark 埋め込みを含められます。front matterがない執筆用Markdownはexportでスキップされます。article は frontmatter の `category` と同名のディレクトリ配下に置く必要があります。category 配下のディレクトリ構造は path から `section_path` として導出され、category page 上の grouped navigation に使われます。
記事が存在するカテゴリでは、対応する`kind: category`のlanding pageが必要です。

## 運用モデル

- VPS 上で Rust 製サーバーバイナリを `systemd` service として起動する
- Topcoat SSR serverはVPSの`127.0.0.1:8008`だけで待ち受ける
- `cloudflared`を`systemd` serviceとして起動し、外向きTunnel経由でCloudflareへ接続する
- HTTPS終端とpublic hostnameはCloudflareで管理し、originの80/443をInternetへ公開しない
- アプリケーション本体は単一バイナリとして扱う
- SSR サーバーは S3 上の成果物を読み、必要に応じて静的ファイルも配信する
- `/api/health` はprocess liveness、`/api/ready` はartifact readerのreadinessとして分ける
- runtimeのAWS認証はIAM Roles AnywhereのX.509 identityと`credential_process`を使い、期限付きrole credentialを取得する
- helper、AWS config、VPS用certificateはroot管理pathへ置き、home directoryには依存しない
- long-livedなIAM user access key、Secrets Manager rotation、credential fileはproduction runtimeに持たない

VPS上のservice設定は[service/README.md](./service/README.md)、Tunnel運用は[Cloudflare Tunnel runbook](./docs/operations/cloudflare-tunnel.md)、IAM Roles Anywhereの検証とcertificate更新は[AWS runtime認証runbook](./docs/operations/aws-runtime-auth.md)を参照してください。

## 開発原則

- `domain` 層は純粋関数のみとし、I/O と `async` を持ち込まない
- 大きめの実装に入る前に GitHub Issue に実装方針とタスク分解を書く
- 実装中の進捗や判断は GitHub Issue / PR に残し、恒久的な知識だけを `docs/architecture/` に昇格する
- 長期的に参照する設計判断は `docs/architecture/` に直接反映する
- `terraform/`は通常のagent作業ではread-onlyとする。repository ownerが明示的に行うinfra変更は、専用の変更計画とplan reviewに従う

## 開発コマンド

タスクランナーは `mise` を使います。タスク定義は [mise.toml](./mise.toml) にあり、一覧は `mise tasks ls` で確認できます。

ローカルでは`mise`だけを事前に導入し、repository rootで次を実行してください。

```bash
mise install
mise run versions-check
```

共通実行tool（Bun、Topcoat CLI）は`mise.toml`をsource of truthとし、`mise.lock`にはmacOS arm64、GitHub Actions Linux x64、VPSが識別するLinux platform aliasの解決済みrelease assetを記録します。Rust toolchainは`rust-toolchain.toml`、Cargo / Bun依存は各manifestとlockfileを正とします。GitHub Actionsは最新majorへ追従し、Renovateでworkflow内のcommit SHAと対応versionのcommentを更新します。

site UIはTopcoat componentとTailwind CSSを主系にします。theme tokenとsite chromeは`crates/server/style/tailwind.css`、artifact由来の生成HTMLは同ファイルからimportする`style/content.css`で管理します。Sass / Stylanceは使用しません。

`mise run export`は公開対象の抽出から記事・タグ・UIの差分翻訳まで実行します。未変更の訳文は再利用し、手動訳を保護して更新候補を自動生成します。候補は`cargo run -p export -- accept article <id>`、`accept tag <id>`、`accept ui <key>`で採用します。手動編集・更新候補・用語集の運用は[export README](crates/export/README.md)を参照してください。`mise run dev-local`は公開Markdownからpublishとlocal配信を行います。原文同期は`mise run sync-obsidian`で明示的に実行し、未commit差分があるsubmoduleは同期しません。
`mise run pull` は deploy 用に `main` の更新だけを行い、submodule も更新したい場合は `mise run pull-with-submodules` を使います。
production CSSはTopcoatのstandalone Tailwind integrationで生成し、そのversionを`mise.toml`の`TOPCOAT_TAILWIND_VERSION`とTopcoat build scriptで一致させます。`mise run versions-check`がこれらとTopcoat CLI / framework、E2EのBun versionを照合し、GitHub Actionsは`jdx/mise-action`経由で同じlocked toolchainを導入します。

共通toolを手動更新するときは、`mise.toml`のversionを更新して`mise lock --platform macos-arm64,linux-x64`を実行します。Bun本体はRenovateの`mise` managerで`mise.toml`と`mise.lock`を更新します。Topcoat本体・CLIとTailwindは手動更新を維持します。Bun package、Rust crate、Rust toolchain、GitHub Actionsの更新もそれぞれの標準manifestと[Renovate設定](./renovate.json)で管理します。GitHub Appの導入、security設定、更新PRの確認は[依存関係の更新](./docs/operations/dependency-updates.md)を参照してください。
browser E2E の依存管理にも Bun を使います。初回は `mise run e2e-install-browser`、実行は `mise run test-e2e` を使ってください。E2E は root の `e2e/` に置き、通常CIではprivate Obsidian submoduleやS3に依存しない固定artifactで実行します。S3への公開はGitHub Actionsの`Publish Content to S3`を`main`から手動実行します。workflowは対象commitのRust CI成功と最新`main`であることを先に確認し、日英のartifactを検証し、同じimmutable releaseを実S3 smoke testで確認します。`publisher_commit`と`content_commit`は公開repositoryの同じcommitを記録します。pointer切替直前にも最新`main`を再確認してから`current.json`を更新します。ローカルからS3へ直接syncする経路は標準の公開手順にしません。

開発端末では、local previewに`mise run dev-local`、S3 readerの本番相当確認に`mise run dev`または`mise run test-e2e-s3`を使います。S3用taskはAWS CLIを実行せず、AWS SDKが設定済みprofileまたは環境変数credentialを読みます。bucketやcredentialは保存せず、`AWS_PROFILE`、region、`OKAWAK_BLOG_ARTIFACT_BUCKET`、必要な場合だけ`OKAWAK_BLOG_ARTIFACT_PREFIX`を実行時に渡します。詳細は[e2e/README.md](./e2e/README.md)を参照してください。

`mise run dev-local`は次を順に行います。`publish`が失敗した場合、Topcoat開発サーバーは起動しません。

- `publish`を通常の厳格モードで実行する
- `crates/publish/dist/site`へartifactを生成する
- Topcoat asset bundleを生成し、`OKAWAK_BLOG_ARTIFACT_SOURCE=local`でTopcoat開発サーバーを起動する

`mise run dev`は次のenvを自動で設定します。

- `OKAWAK_BLOG_ARTIFACT_SOURCE=s3`
- `OKAWAK_BLOG_SITE_ORIGIN=http://127.0.0.1:8008`

`OKAWAK_BLOG_ARTIFACT_BUCKET`は必須で、任意のprefixやAWS credentialとともに実行時に渡します。固定fixtureを使う`test-e2e`は、外部状態に依存しないCI回帰テストとして別に維持します。`mise run build-project`はdeploy用のbuildで、artifactやprivate submoduleには依存しません。

production deployは`mise run build-deployment`で`target/release/server`と`target/assets-staged`を生成します。`mise run quick-deploy`はservice停止中にbinaryとcontent-hash付きasset bundleを同じreleaseへ切り替え、health / readinessが失敗した場合は両方を旧releaseへ戻します。

通常の更新は管理端末から`mise run deploy-vps`を実行します。SSH経由でVPS内部の`production-deploy` taskを呼び、最新mainの取得・ビルド・配備をVPSで行います。VPSにもmiseとGit管理下の設定が必要ですが、`mise.local.toml`は不要です。

Topcoat asset bundleはTailwind CSS、Topcoat runtime、faviconをcontent-hash付きlocal URLで配信します。公開linkはブラウザ標準のfull-page navigationを使います。GitHubアイコンは同梱したOcticonsのSVGをTopcoatのicon componentでinline描画します。端末間で字体を揃えるためNoto Sans JPをGoogle Fontsから読み込み、400〜700の可変ウェイト指定でCSSの重複を抑えます。フォントCSSはHTML headから直接参照し、`display=swap`で読み込み中も本文を表示します。生成コンテンツの描画に必要なKaTeXとhighlight.jsはversion固定の外部CDN資産として維持します。KaTeXはSRIを付与し、いずれもsiteのSSR可用性を左右する必須runtimeにはしません。

主要コマンドは以下です。

```bash
mise run check-deps
mise run versions-check
mise run sync-obsidian
mise run pull-with-submodules
mise run dev
mise run dev-local
mise run format
mise run e2e-install-browser
mise run test
mise run test-domain
mise run test-server
mise run test-e2e
mise run test-e2e-s3
mise run clippy
mise run check
```

デプロイ・運用taskは管理端末からSSH経由で実行します。VPS内部用の配備taskは通常の一覧から隠しています。

```bash
mise run deploy-vps
mise run status-vps
mise run logs-vps
mise run logs-recent-vps
mise run restart-vps
```

## 多言語運用の移行と確認

1. 依存するPRを順に統合し、通常CIを通す。公開Markdownのpush時点でGitHub上では文章が公開されるため、push前に対象・訳文・辞書の差分を確認する。
2. `mise run export`で原文から公開版を同期し、必要な英訳を生成する。英訳は手で修正でき、原文更新時は手動訳を保護して更新候補を自動生成する。操作は[export README](crates/export/README.md)を参照する。
3. `mise run dev-local`で日英を確認し、Markdownと辞書だけをGitへ確定する。未作成・更新待ちの英訳は英語一覧から外れ、英語URLは404になる。日本語の公開は継続できる。
4. 最初にserver binaryとassetを配備する。新serverは旧releaseも日本語として読める。次に`Publish Content to S3`をmainから実行する。旧serverは新releaseの日本語を読めるが、英語ルートと新しいUIはserver更新後に利用できる。

タグ表示名は記事と同じreleaseで更新されます。UIの定型文は[server辞書](crates/server/locales/README.md)をbinaryへ組み込むため、UI変更時はserverの再配備も必要です。英語homeの正規URLは`/en`で、`/en/`は既存のslash規則に従いredirectします。

トップページ`/`への初回アクセスではブラウザの`Accept-Language`を使い、日本語を優先する環境は日本語、それ以外・未指定は英語homeへ案内します。上部の「日本語 / English」で変更すると選択を1年間cookieへ保存し、以後はブラウザの設定より優先します。記事への直接アクセスはURLの言語を維持します。切替先の記事が未翻訳なら対象言語のhomeへ移り、その言語自体が未公開なら選択を無効表示して日本語を継続します。JavaScript無効時も切り替えられます。
