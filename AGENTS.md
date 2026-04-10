# AGENTS.md

## 会話ガイドライン

- 常に日本語で会話する

## このリポジトリの位置付け

`okawak_blog` は、Rust 製のブログ CMS を作るリポジトリではありません。Obsidian で書いた Markdown を公開成果物へ変換し、それを Leptos SSR で配信するための、静的コンテンツ公開基盤 + SSR 表示基盤として扱います。

主役は常駐 API サーバーではなく、コンテンツ生成パイプラインです。

## 優先して参照する文書

1. `docs/architecture/re-architecture.md`
2. GitHub Issue / PR
3. `README.md`

### 参照ルール

- 再設計方針の一次情報は `docs/architecture/re-architecture.md`
- 個別作業の進め方と進捗は GitHub Issue / PR を優先する
- 長期的に残す設計判断は `docs/architecture/` に集約する
- `README.md` は最終的な目標像の概要説明であり、現行実装の一次情報としては扱わない

## 現在の構成と目標構成を混同しない

### 現在の workspace

```text
okawak_blog/
├── crates/
│   ├── domain/
│   ├── publish/
│   │   ├── publisher/
│   │   ├── artifacts/
│   │   ├── bookmark/
│   │   └── ingest/
│   └── site/
│       ├── infra/
│       ├── server/
│       └── web/
├── docs/
├── service/
└── terraform/
```

### 目標構成

`docs/architecture/re-architecture.md` では、将来的に以下への再編を目指している。

```text
okawak_blog/
├── crates/
│   ├── domain/
│   ├── publish/
│   │   ├── publisher/
│   │   ├── ingest/
│   │   ├── artifacts/
│   │   └── bookmark/
│   └── site/
│       ├── infra/
│       ├── web/
│       └── server/
├── docs/
│   └── architecture/
└── ...
```

### 文書化と実装の注意

- 未作成の crate / app を、現在存在するものとして書かない
- 「現在の構成」と「移行先の目標構成」を必ず分けて説明する
- README や設計メモを書くときも、理想像を現状の事実として断定しない

## アーキテクチャ原則

### コンテンツパイプライン中心

- Obsidian の Markdown を読み取る
- Front Matter を検証する
- 内部リンクや埋め込みを解決する
- HTML / index JSON を生成する
- S3 に成果物を配置する
- Leptos SSR サーバーがそれを読んで配信する

### Obsidian ソースの扱い

- 記事ソースの source of truth は private な別 Obsidian リポジトリ
- `obsidian` の Markdown はこの public リポジトリへ通常ファイルとして commit しない
- publisher は git submodule として取得した Obsidian repo を入力に使う
- ローカル開発でも GitHub Actions でも submodule を初期化・更新してから publisher を実行する

### Markdown 変換はビルド時

- Markdown をリクエスト時に毎回変換しない
- HTML 生成は publisher 側に寄せる
- SSR サーバーは、成果物読取・ルーティング・メタ情報付与に集中する

### Rust らしい責務分割

- `domain` は純粋関数と小さな型を中心にする
- `domain` は I/O を知らない
- `domain` は `async` 前提にしない
- `domain` は AWS SDK、Leptos、Axum を知らない
- 外部境界は trait で薄く切る
- 単一バイナリでの本番運用を優先する

### publisher 側と reader 側の配置境界

- Obsidian 読み取り、Front Matter 解析、Markdown 変換、成果物生成など publisher 専用の実装は `crates/publish/` に置く
- `crates/site/infra` は Leptos サーバーが公開成果物を読むための infrastructure に限定する
- `crates/site/infra` に publisher 側の vault reader、Markdown renderer、upload 実装を置かない
- publisher と reader の両方で共有する純粋な契約やルールだけを `crates/domain` に置く

## 非目標

明示されない限り、以下は作らない前提で考える。

- DB ベースの記事管理
- ユーザー認証・認可
- 管理画面
- UI からの記事作成・編集
- マルチユーザー機能
- SaaS 的 CMS 機能
- リアルタイム更新基盤

## 各ディレクトリの扱い

### `crates/domain`

- 純粋ドメインロジックを置く
- I/O 禁止
- `async/await` 禁止
- WASM 互換を意識する
- publisher と reader で共有する公開成果物契約はここで扱う
- artifact から組み立てる site page contract のような pure model もここで扱う

### `crates/site/infra`

- Leptos サーバー側の infrastructure 専用として扱う
- 現在は `ArtifactReader` 境界の first cut として local / S3 artifact reader を置いている
- local reader は dev / test 用、S3 reader は本番読取経路として扱う
- 将来的な責務は S3 読み取り、キャッシュ、設定読込など reader 側の外部境界
- Obsidian vault 読み取り、Front Matter parse、Markdown render、S3 upload はここへ置かない

### `crates/site/server`

- 現在のサーバー実装
- 将来的には SSR 公開用途に責務を絞る想定
- S3 上の公開成果物を読む blog 側の中心として扱う

### `crates/site/web`

- Leptos UI / SSR ルーティング層
- 公開成果物を読む側として整理する

### `crates/publish/publisher`

- 現在もっとも `publisher` に近いアプリ
- 今後の再設計では公開成果物生成の主役として育てる前提で扱う
- ingest / artifacts / bookmark など publisher 専用の補助 crate を `crates/publish/` 配下へ置く
- 入力となる Obsidian Markdown は private repo の git submodule から取得する

### `crates/publish/artifacts`

- publisher 側の artifact 組み立てとローカル書き出しを担う補助 crate
- `publisher` から切り出した publisher 専用ロジックの受け皿として扱う

### `crates/publish/ingest`

- Obsidian vault の走査、Front Matter 解析、Markdown 変換を担う補助 crate
- `publisher` から切り出した publisher 入力処理の受け皿として扱う

### `crates/publish/bookmark`

- 外部 HTTP を伴う bookmark enrichment を担う補助 crate
- OGP 取得、bookmark 変換、将来的な retry や cache などをここへ寄せる

### `service`

- `systemd`、`nginx`、運用補助ファイルを置く

### `terraform`

- 読み取りのみ
- 編集禁止
- このディレクトリではコマンドを実行しない

## 開発プロセス

### 実装前の準備

- 大きめの実装に入る前に GitHub Issue を作成または更新する
- Issue には実装方針、依存方向、各層の責務、タスク分解を書く
- 具体的なコード断片を先に恒久ドキュメントへ書き込みすぎない
- 恒久的に残す価値がある内容だけを `docs/architecture/` に反映する

### TDD

- 可能な限り TDD で進める
- 純粋ロジックは先にテストを書く
- 実装中は、仕様変更でない限りテストを都合よく変えない

### 文書更新

- 責務分割や依存方向を変えたら README / AGENTS / `docs/architecture/` を更新する
- 文書を更新する際は、現状説明と将来方針を分ける
- 実在しない構成を、現行実装として書かない
- 実装計画や進捗メモを repo 内 docs に増やし続けない

## コーディングと設計上の注意

- 過剰な repository パターンを持ち込まない
- `shared` 的な置き場を安易に肥大化させない
- ビルド時に解決できる責務をサーバーランタイムへ持ち込まない
- `unimplemented!()` 前提の大きなモデルを増やさない
- 型で状態遷移を表せるなら優先する

## 実行コマンドの目安

- タスクランナーは `mise` を使う
- task 定義は repo root の `mise.toml`
- `mise` 経由のローカル task では、`OKAWAK_BLOG_ARTIFACT_SOURCE=local` と `OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT=crates/publish/publisher/dist/site` を使って local artifact を読む
- local publisher 実行前には `mise run sync-obsidian` 相当で private Obsidian submodule を初期化する
- `mise run pull` は deploy 用に main の更新だけを行い、submodule 更新が必要なときだけ `mise run pull-with-submodules` を使う
- `crates/site/web/package.json` の依存操作は root から `mise run web-install` / `mise run web-update` / `mise run web-outdated` を使う
- 同一ネットワークの別端末から確認する一時用途では `mise run dev-lan` を使い、必要なら `OKAWAK_BLOG_SITE_ORIGIN=http://<host-ip>:8008` を前置して absolute URL を合わせる
- 本番 runtime は `service/okawak_blog.service` 側の env により `s3` reader を使う

### 開発

- `mise run check-deps`
- `mise run sync-obsidian`
- `mise run pull-with-submodules`
- `mise run publish-local`
- `mise run dev`
- `mise run dev-lan`
- `mise run integrated-dev`
- `mise run watch`
- `mise run format`
- `mise run build-local`
- `mise run web-install`
- `mise run web-update`
- `mise run web-outdated`

### テスト・確認

- `mise run test`
- `mise run test-domain`
- `mise run test-server`
- `mise run test-web`
- `mise run clippy`
- `mise run check`
- `mise run check-domain`
- `mise run check-server`

### デプロイ・運用

- `mise run build-project`
- `mise run pull`
- `mise run full-deploy`
- `mise run production-deploy`
- `mise run status`
- `mise run logs`
- `mise run logs-recent`

`sudo` を伴うタスクは、ローカル開発環境ではなく VPS 前提で扱う。
