[![Sync Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

`okawak_blog` は、Obsidian で書いた Markdown を Rust 製の publisher が公開成果物へ変換して S3 に配置し、それを VPS 上の単一バイナリ Leptos SSR サーバーが nginx 配下で公開する、静的コンテンツ公開基盤 + SSR 表示基盤です。

## 関連文書

- [docs/architecture/architecture.md](./docs/architecture/architecture.md): 現行アーキテクチャと artifact 契約
- [docs/content/obsidian-template.md](./docs/content/obsidian-template.md): Obsidian Markdown のテンプレート
- GitHub Issues / PRs: 実装計画、進捗、作業単位の管理

## このリポジトリが担うこと

- 記事は Obsidian で執筆する
- 記事ソースは private な Obsidian リポジトリで管理する
- 記事ソースはこの public リポジトリへ直接 commit せず、git submodule として参照する
- GitHub Actions またはローカル実行の publisher が公開成果物を生成する
- 生成した HTML / index JSON を S3 に配置する
- Leptos SSR サーバーが S3 上の成果物を読んで配信する
- VPS + `systemd` + `nginx` で単純に運用できる構成を保つ

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

1. Obsidian の Markdown を読む
2. Front Matter を解釈・検証する
3. 内部リンクや埋め込みを解決する
4. Markdown を公開用 HTML に変換する
5. 記事一覧やカテゴリ一覧などの index データを生成する
6. 成果物を S3 にアップロードする
7. Leptos SSR サーバーがそれを読んで公開する

この境界に合わせて、publisher 側の実装は `crates/publish/` に、公開成果物を読む blog 側の実装は `crates/site/` に寄せます。`crates/domain/` は両者で共有する契約と純粋ルールを置く場所として扱います。

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
│   ├── publish/
│   │   ├── publisher/        # publisher CLI の中心
│   │   ├── ingest/           # Obsidian 入力の走査・解析・変換
│   │   ├── artifacts/        # 成果物 bundle の組み立てと local writer
│   │   └── bookmark/         # bookmark enrichment
│   └── site/
│       ├── infra/            # Leptos サーバー側の S3 / cache / runtime adapter
│       ├── server/           # 公開成果物を読む統合バックエンド
│       └── web/              # Leptos SSR 公開 UI
├── e2e/                      # 公開サイト全体の browser E2E
├── docs/
│   └── architecture/
├── service/
└── terraform/
```

### 各層の責務

- `crates/domain`: 公開成果物契約、site page contract、純粋関数を中心にした共有ドメイン層
- `crates/publish/*`: Obsidian 読み取り、Markdown 変換、成果物生成を担う publisher 側
- `crates/site/infra`: Leptos サーバーが公開成果物を読むための S3 / cache / runtime adapter。開発と本番はS3 readerを使い、local readerは自動test用に残す
- `crates/site/server`: S3 上の成果物を読んで配信し、release-aware ETag / Last-Modifiedを扱う統合バックエンド
- `crates/site/web`: Leptos SSR の公開 UI
- `e2e`: server / web / artifact reader をまたぐ、固定 artifact ベースの browser E2E

## 公開成果物のイメージ

```text
site/
├── articles/
│   ├── <category>/
│   │   └── <slug>.html
│   └── index.json
├── categories/
│   ├── <category>/
│   │   ├── index.json
│   │   └── page.html
│   └── ...
├── pages/
│   ├── about.json
│   ├── home.json
│   └── ...
└── metadata/
    └── site.json
```

publisher はこれらの成果物を生成し、SSR サーバーはそれらを読んでページを返します。

## データフロー

```text
Obsidian repo
  -> git submodule
  -> publisher
  -> HTML / index JSON を生成
  -> AWS S3
  -> Leptos SSR server
  -> Browser
```

publisher が読む `obsidian` の Markdown は、この public repo へ通常ファイルとして同梱しない。source of truth は private な別リポジトリであり、ローカル開発と GitHub Actions の両方で git submodule として取得する。

## Obsidian Front Matter

Publisher が扱う Markdown には、YAML front matter が必要です。現在の parser は LF 区切りの front matter を前提にしており、`is_completed: true` のものだけを公開対象として扱います。役割判定には `kind` を使います。

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

本文は closing `---` の次の行から始まり、Obsidian link や bookmark 埋め込みを含められます。front matter がない Markdown は publisher からはスキップされます。category 配下のディレクトリ構造は path から `section_path` として導出され、category page 上の grouped navigation に使われます。

## 運用モデル

- VPS 上で Rust 製サーバーバイナリを `systemd` service として起動する
- `nginx` を前段に置いて HTTPS 終端とリバースプロキシを担当させる
- アプリケーション本体は単一バイナリとして扱う
- SSR サーバーは S3 上の成果物を読み、必要に応じて静的ファイルも配信する
- `/api/health` はprocess liveness、`/api/ready` はartifact readerのreadinessとして分ける
- runtime用AWS credentialsは`/var/lib/okawak_blog/aws/credentials`へ置き、home directoryには依存しない

VPS上のservice設定とcredential更新手順は[service/README.md](./service/README.md)を参照してください。

## 開発原則

- `domain` 層は純粋関数のみとし、I/O と `async` を持ち込まない
- 大きめの実装に入る前に GitHub Issue に実装方針とタスク分解を書く
- 実装中の進捗や判断は GitHub Issue / PR に残し、恒久的な知識だけを `docs/architecture/` に昇格する
- 長期的に参照する設計判断は `docs/architecture/` に直接反映する
- `terraform/` は読み取り専用とし、編集やコマンド実行を行わない

## 開発コマンド

タスクランナーは `mise` を使います。タスク定義は [mise.toml](./mise.toml) にあり、一覧は `mise tasks ls` で確認できます。

ローカルでは`mise`だけを事前に導入し、repository rootで次を実行してください。

```bash
mise install
mise run versions-check
```

共通実行tool（Bun、cargo-leptos、leptosfmt）は`mise.toml`をsource of truthとし、`mise.lock`にはmacOS arm64とGitHub Actions Linux x64の解決済みrelease assetを記録します。Rust toolchainは`rust-toolchain.toml`、Cargo / Bun依存は各manifestとlockfile、GitHub Actionsはworkflow内の最新major指定を正とします。

web UIはRust/UI由来のprimitiveとTailwind CSSを主系にします。theme tokenとsite chromeは`crates/site/web/style/tailwind.css`、artifact由来の生成HTMLは同ファイルからimportする`style/content.css`で管理します。Sass / Stylanceは使用しません。

private Obsidian repoを使うpublisher側の開発では、必要なときだけ`mise run sync-obsidian`でsubmoduleを同期します。開発サーバーの表示確認ではlocal artifactを生成せず、GitHub Actionsが公開したS3 artifactを読みます。
`mise run pull` は deploy 用に `main` の更新だけを行い、submodule も更新したい場合は `mise run pull-with-submodules` を使います。
`crates/site/web/package.json` の依存のインストール/更新確認は root から `mise run web-install` / `mise run web-update` / `mise run web-outdated` で行えます。

`cargo-leptos`が取得するTailwind CLIのバージョンは、`mise.toml`の`LEPTOS_TAILWIND_VERSION`で固定します。Bun管理のTailwind依存は`crates/site/web/package.json`を正とし、`mise run versions-check`が両者とE2EのBun versionを照合します。GitHub Actionsは`jdx/mise-action`経由で同じlocked toolchainを導入します。

共通toolを更新するときは、`mise.toml`のversionを更新して`mise lock --platform macos-arm64,linux-x64`を実行します。Bun package、Rust crate、Rust toolchain、GitHub Actionsの更新はそれぞれの標準manifestとDependabotで管理します。
browser E2E の依存管理にも Bun を使います。初回は `mise run e2e-install-browser`、実行は `mise run test-e2e` を使ってください。E2E は root の `e2e/` に置き、通常CIではprivate Obsidian submoduleやS3に依存しない固定artifactで実行します。upload workflowはimmutable releaseを実S3 smoke testで検証し、成功後だけ`current.json`を切り替えます。

開発端末での表示確認は、S3 readerを使う`mise run dev`または`mise run test-e2e-s3`を標準とします。taskはAWS CLIを実行せず、AWS SDKが設定済みprofileまたは環境変数credentialを読みます。bucketやcredentialは保存せず、`AWS_PROFILE`、region、`OKAWAK_BLOG_ARTIFACT_BUCKET`、必要な場合だけ`OKAWAK_BLOG_ARTIFACT_PREFIX`を実行時に渡します。詳細は[e2e/README.md](./e2e/README.md)を参照してください。

`mise run dev`は次のenvを自動で設定します。

- `OKAWAK_BLOG_ARTIFACT_SOURCE=s3`
- `OKAWAK_BLOG_SITE_ORIGIN=http://127.0.0.1:8008`

`OKAWAK_BLOG_ARTIFACT_BUCKET`は必須で、任意のprefixやAWS credentialとともに実行時に渡します。local artifact readerを使う開発用mise taskは提供しません。固定fixtureを使う`test-e2e`は、外部状態に依存しないCI回帰テストとして別に維持します。`mise run build-project`はdeploy用のbuildで、artifactやprivate submoduleには依存しません。

主要コマンドは以下です。

```bash
mise run check-deps
mise run versions-check
mise run sync-obsidian
mise run pull-with-submodules
mise run dev
mise run format
mise run web-install
mise run web-update
mise run web-outdated
mise run e2e-install-browser
mise run test
mise run test-domain
mise run test-server
mise run test-web
mise run test-e2e
mise run test-e2e-s3
mise run clippy
mise run check
```

VPS 前提のデプロイ・運用タスクも `mise` に移しています。

```bash
mise run pull
mise run build-project
mise run full-deploy
mise run production-deploy
mise run status
mise run logs
mise run logs-recent
```
