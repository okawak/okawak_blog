# AGENTS.md

## 会話とGit

- 常に日本語で簡潔に会話する
- commit は署名付きで作る。commit 前に署名設定を確認する
- ユーザーの既存差分を無断で破棄・上書きしない

## リポジトリの位置付け

`okawak_blog` は、private な Obsidian Markdown を公開 artifact へ変換し、Leptos SSR で配信する静的コンテンツ公開基盤である。主役は常駐 API や CMS ではなく、ビルド時のコンテンツ生成パイプラインである。

参照優先順位:

1. `docs/architecture/architecture.md`（現行設計の一次情報）
2. GitHub Issue / PR（個別計画と進捗）
3. `README.md`（概要と利用方法）

実在しない構成を現行実装として記述しない。実装計画は Issue / PR に置き、恒久文書には現在有効な設計判断だけを残す。

## 必須アーキテクチャ原則

- source of truth は private Obsidian repository。public repository へ記事 Markdown を通常ファイルとして commit しない
- publisher は git submodule の Obsidian repository を入力にする。必要な作業時だけ submodule を初期化・更新する
- Markdown / frontmatter / link / embed の解決と HTML 生成はビルド時に行う
- SSR runtime は公開 artifact の読取、ルーティング、メタ情報付与に集中する
- production は単一 server binary を優先する

依存と配置の境界:

- `crates/domain`: publisher と reader が共有する純粋な型・契約・ルール。I/O、`async`、AWS SDK、Axum、Leptos を持ち込まない。WASM 互換を意識する
- `crates/publish/ingest`: vault 走査、frontmatter、Markdown変換、Obsidian記法の解決
- `crates/publish/artifacts`: artifact の組み立てとローカル書出し
- `crates/publish/bookmark`: 外部 HTTP を伴う bookmark enrichment
- `crates/publish/publisher`: publish 処理の orchestration。publisher 専用処理は `crates/publish/` に置く
- `crates/site/infra`: server が artifact を読む外部境界（local / S3、設定、将来のcache）。vault読取、Markdown変換、uploadを置かない
- `crates/site/server`: Axum + Leptos SSR host、reader注入、API、health/readiness
- `crates/site/web`: Leptos UI / route / metadata。SSR時もstorage実装へ直接依存しない
- `e2e`: repository root直下のbrowser E2E。通常CIはprivate submoduleやAWSに依存しないfixtureで検証し、実S3の手動smoke testは専用configへ分離する
- `service`: systemd、nginx、運用補助
- `terraform`: 読み取り専用。編集せず、このdirectoryでcommandを実行しない

`domain`、`publish`、`site` の責務をまたぐ純粋契約だけを `domain` へ置く。publisher専用処理を reader 側へ移さず、ビルド時に解決できる責務をruntimeへ持ち込まない。

## 非目標

明示されない限り、DBベースの記事管理、認証認可、管理画面、UI編集、マルチユーザー、SaaS CMS、リアルタイム更新基盤は作らない。

## 開発プロセス

- 大きめの実装前に GitHub Issue を作成または更新し、目的、責務、依存方向、タスク、受け入れ条件を書く
- 可能な限りTDDで進め、純粋ロジックは失敗テストを先に置く。仕様変更でない限りテストを都合よく変更しない
- 責務や依存方向を変えたら `docs/architecture/` と必要な利用文書を更新する
- 過剰なrepository pattern、肥大化する`shared`、`unimplemented!()`前提の大きなmodelを避ける
- 型で状態遷移や不変条件を表せる場合は優先する
- GitHub Actionsは原則として利用中actionの最新majorを指定する

## タスクと運用

タスクランナーはrepository rootの`mise.toml`を正とする。利用可能なtaskは`mise tasks ls`で確認し、直接commandを複製せず`mise run <task>`を優先する。

主要な確認:

- `mise run format`
- `mise run test`
- `mise run clippy`
- `mise run check`
- `mise run test-e2e`
- `mise run test-e2e-s3`（明示的な実S3手動確認のみ）

開発サーバーはS3 readerを標準とし、`mise run dev`で次を使う。

- `OKAWAK_BLOG_ARTIFACT_SOURCE=s3`
- `OKAWAK_BLOG_ARTIFACT_BUCKET`（実行時に必須）

publisher側の作業で必要な場合だけ`mise run sync-obsidian`を使う。local artifact readerは自動test fixtureに限定し、開発用mise taskを追加しない。web / E2Eの依存操作もrootの`web-*` / `e2e-*` taskを使う。S3の手動確認は`dev` / `test-e2e-s3`を使い、本番runtimeのS3設定とcredentialsは`service/okawak_blog.service`および`service/README.md`を参照する。

- `/api/health`: process liveness
- `/api/ready`: artifact reader readiness
- `sudo`を伴うtaskはVPS運用向けとして扱う
