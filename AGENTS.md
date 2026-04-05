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
│   ├── server/
│   └── web/
├── apps/
│   ├── obsidian_uploader/
│   └── publisher_artifacts/
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
│   ├── infrastructure/
│   ├── web/
│   └── server/
├── apps/
│   ├── obsidian_uploader/
│   └── ...                # publisher 側の補助 crate 群
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

- Obsidian 読み取り、Front Matter 解析、Markdown 変換、成果物生成、S3 アップロードなど publisher 専用の実装は `apps/` に置く
- `crates/infrastructure` は Leptos サーバーが公開成果物を読むための infrastructure に限定する
- `crates/infrastructure` に publisher 側の vault reader、Markdown renderer、upload 実装を置かない
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

### `crates/infrastructure`

- 将来的に導入または拡張する場合も、Leptos サーバー側の infrastructure 専用として扱う
- 想定する責務は S3 読み取り、キャッシュ、設定読込など reader 側の外部境界
- Obsidian vault 読み取り、Front Matter parse、Markdown render、S3 upload はここへ置かない

### `crates/server`

- 現在のサーバー実装
- 将来的には SSR 公開用途に責務を絞る想定
- S3 上の公開成果物を読む blog 側の中心として扱う

### `crates/web`

- Leptos UI / SSR ルーティング層
- 公開成果物を読む側として整理する

### `apps/obsidian_uploader`

- 現在もっとも `publisher` に近いアプリ
- 今後の再設計では公開成果物生成の主役として育てる前提で扱う
- parser / renderer / uploader など publisher 専用の補助 crate を切る場合も `apps/` 配下へ置く
- 入力となる Obsidian Markdown は private repo の git submodule から取得する

### `apps/publisher_artifacts`

- publisher 側の artifact 組み立てとローカル書き出しを担う補助 crate
- `obsidian_uploader` から切り出した publisher 専用ロジックの受け皿として扱う

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

### 開発

- `cargo make dev`
- `cargo make integrated-dev`
- `cargo make watch`
- `cargo make format`

### テスト・確認

- `cargo make test`
- `cargo make test-domain`
- `cargo make test-server`
- `cargo make test-web`
- `cargo make clippy`
- `cargo make check`
- `cargo make check-domain`
- `cargo make check-server`

### デプロイ・運用

- `cargo make build-project`
- `cargo make full-deploy`
- `cargo make production-deploy`
- `cargo make status`
- `cargo make logs`
- `cargo make logs-recent`

`sudo` を伴うタスクは、ローカル開発環境ではなく VPS 前提で扱う。
