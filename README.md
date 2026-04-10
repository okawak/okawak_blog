[![Sync Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

`okawak_blog` は、Obsidian で書いた Markdown を Rust 製の publisher が公開成果物へ変換して S3 に配置し、それを VPS 上の単一バイナリ Leptos SSR サーバーが nginx 配下で公開する、静的コンテンツ公開基盤 + SSR 表示基盤です。

## 関連文書

- [docs/architecture/re-architecture.md](./docs/architecture/re-architecture.md): 目標アーキテクチャへ移行するための設計メモ
- GitHub Issues / PRs: 実装計画、進捗、作業単位の管理

## このリポジトリが担うこと

- 記事は Obsidian で執筆する
- 記事ソースは private な Obsidian リポジトリで管理する
- 記事ソースはこの public リポジトリへ直接 commit せず、git submodule として参照する
- GitHub Actions またはローカル実行の publisher が公開成果物を生成する
- 生成した HTML / index JSON / assets を S3 に配置する
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

## 最終的な workspace 像

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
├── docs/
│   └── architecture/
├── service/
└── terraform/
```

### 各層の責務

- `crates/domain`: 公開成果物契約、site page contract、純粋関数を中心にした共有ドメイン層
- `crates/publish/*`: Obsidian 読み取り、Markdown 変換、成果物生成を担う publisher 側
- `crates/site/infra`: Leptos サーバーが公開成果物を読むための S3 / cache / runtime adapter。local reader は dev / test 用に残し、本番は S3 reader を使う
- `crates/site/server`: S3 上の成果物を読んで配信する統合バックエンド
- `crates/site/web`: Leptos SSR の公開 UI

## 公開成果物のイメージ

```text
site/
├── articles/
│   ├── <slug>.html
│   └── index.json
├── categories/
│   ├── tech.json
│   ├── daily.json
│   └── ...
├── tags/
│   └── index.json
├── assets/
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
  -> HTML / index JSON / assets を生成
  -> AWS S3
  -> Leptos SSR server
  -> Browser
```

publisher が読む `obsidian` の Markdown は、この public repo へ通常ファイルとして同梱しない。source of truth は private な別リポジトリであり、ローカル開発と GitHub Actions の両方で git submodule として取得する。

## Obsidian Front Matter

Publisher が記事として扱う Markdown には、YAML front matter が必要です。現在の parser は LF 区切りの front matter を前提にしており、`is_completed: true` の記事だけを公開対象として扱います。

```yaml
---
title: "Rust Performance Notes"
tags: ["rust", "performance"]
summary: "Short summary shown in lists and metadata."
is_completed: true
priority: 1
created: "2025-01-15T10:00:00+09:00"
updated: "2025-01-16T09:30:00+09:00"
category: "tech"
---
```

各フィールドの役割は次の通りです。

- `title`: 記事タイトル。必須です。
- `tags`: タグ一覧。省略可能です。
- `summary`: 一覧やメタ情報に使う短い説明。省略可能です。
- `is_completed`: 公開対象かどうかを示すフラグ。`true` の記事だけを出力します。
- `priority`: 並び順や強調表示に使う優先度。省略可能です。
- `created`: 作成日時。必須です。
- `updated`: 更新日時。必須です。
- `category`: 記事カテゴリ。省略時はカテゴリなしとして扱います。

本文は closing `---` の次の行から始まり、Obsidian link や bookmark 埋め込みを含められます。front matter がない Markdown は publisher からはスキップされます。

## 運用モデル

- VPS 上で Rust 製サーバーバイナリを `systemd` service として起動する
- `nginx` を前段に置いて HTTPS 終端とリバースプロキシを担当させる
- アプリケーション本体は単一バイナリとして扱う
- SSR サーバーは S3 上の成果物を読み、必要に応じて静的ファイルも配信する

## 開発原則

- `domain` 層は純粋関数のみとし、I/O と `async` を持ち込まない
- 大きめの実装に入る前に GitHub Issue に実装方針とタスク分解を書く
- 実装中の進捗や判断は GitHub Issue / PR に残し、恒久的な知識だけを `docs/architecture/` に昇格する
- 長期的に参照する設計判断は `docs/architecture/` に直接反映する
- `terraform/` は読み取り専用とし、編集やコマンド実行を行わない

## 開発コマンド

タスクランナーは `mise` を使います。タスク定義は [mise.toml](./mise.toml) にあり、一覧は `mise tasks ls` で確認できます。

ローカル開発に必要なツールは以下です。

- `mise`
- `bun`
- `cargo-leptos`
- `leptosfmt`

```bash
cargo install cargo-leptos
cargo install leptosfmt
```

スタイリング用のツールチェーンは別途検討中です。方針は Issue `#39` で整理し、確定後にセットアップ手順へ反映します。
ただし、現時点の web build task は既存の `stylance` コマンドを前提にしています。

また、private Obsidian repo を入力として使うため、local artifact を再生成したいときは `mise run publish-local` を実行します。`publish-local` は内部で `git submodule update --init --recursive` を実行しますが、先に明示的に同期したい場合は `mise run sync-obsidian` を使えます。
`mise run pull` は deploy 用に `main` の更新だけを行い、submodule も更新したい場合は `mise run pull-with-submodules` を使います。
`crates/site/web/package.json` の依存のインストール/更新確認は root から `mise run web-install` / `mise run web-update` / `mise run web-outdated` で行えます。

ローカル開発用 task では、次の env を自動で設定します。

- `OKAWAK_BLOG_ARTIFACT_SOURCE=local`
- `OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT=crates/publish/publisher/dist/site`
- `OKAWAK_BLOG_SITE_ORIGIN=http://127.0.0.1:8008`

そのため、`mise run dev` や `mise run build-local` のようなローカル task は、S3 ではなく publisher が生成した local artifact を読む前提で動作します。開発サーバー系 task は artifact を自動再生成しないので、記事内容を更新した後や初回セットアップ時は先に `mise run publish-local` を実行してください。`mise run build-project` は deploy 用の build で、local artifact や private submodule には依存しません。

同じネットワーク上の別端末から動作確認したい場合は `mise run dev-lan` を使います。これは `LEPTOS_SITE_ADDR=0.0.0.0:8008` で待ち受けるだけの task で、別端末からはホストの IP アドレスを直接指定してアクセスします。absolute URL までその IP に揃えたい場合だけ、次のように host IP を明示して実行します。

```bash
OKAWAK_BLOG_SITE_ORIGIN=http://<host-ip>:8008 mise run dev-lan
```

主要コマンドは以下です。

```bash
mise run check-deps
mise run sync-obsidian
mise run pull-with-submodules
mise run publish-local
mise run dev
mise run dev-lan
mise run integrated-dev
mise run watch
mise run format
mise run web-install
mise run web-update
mise run web-outdated
mise run test
mise run test-domain
mise run test-server
mise run test-web
mise run clippy
mise run check
mise run build-local
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
