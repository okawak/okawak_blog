# AGENTS.md

## 基本ルール

- 日本語で簡潔・丁寧に会話する。
- commit前に署名設定を確認し、署名付きで作る。既存差分を無断で破棄・上書きしない。
- PRのreview threadに対応したら、修正をpushしてから該当threadをresolveする。
- `terraform/`は読み取り専用。編集も、このdirectory内でのcommand実行もしない。

## 設計と責務

private Obsidian Markdownをビルド時の`publish` pipelineで公開artifactへ変換し、Topcoat SSRで配信する基盤。
設計・module構成は[architecture](docs/architecture/architecture.md)に従う。参照優先順位は同文書 → GitHub Issue / PR → [README](README.md)。計画・進捗はIssue / PRに置き、恒久文書には現行の設計だけを書く。

- 正本はprivate Obsidian repository。記事Markdownをpublic repositoryへ通常ファイルとしてcommitしない。入力のgit submoduleは必要時だけ初期化・更新する。
- Markdown / frontmatter / link / embedの解決とHTML生成はビルド時に完了させる。SSRはartifact読取・routing・metadata付与に集中し、本番は単一server binaryを優先する。
- `crates/domain`: 公開コンテンツの純粋model・ルールとcrate間の共有契約。I/O、`async`、AWS SDK、HTTP frameworkを持ち込まない。
- `crates/publish`: 入力・変換・artifact生成を担う単一crate。`lib.rs`はmodule宣言とre-exportのみ。外部APIはpublish entrypoint、bookmark enricher注入、`PublishError` / `Result`に限定し、内部の責務分割は設計文書に従う。
- `crates/site/infra`: local / S3のartifact読取・設定・cache境界。vault読取・Markdown変換・uploadを置かない。
- `crates/site/server`: 単一Topcoat application。UI・metadata・asset・reader注入・API・health/readiness・conditional GETを所有する。`src/app.rs`を`module_router!()`のrootとし、`app/`のfile moduleをURL構造に対応させる（`mod.rs`禁止）。UIはstorage実装へ直接依存せず`PageLoader`を経由する。
- `e2e/`: browser E2E。通常CIはprivate submodule・AWS不要のfixtureを使い、実S3 smokeはローカル手動確認とupload workflowの公開前gateで行う。
- 明示されない限り、DB記事管理・認証認可・管理画面・UI編集・マルチユーザー・SaaS CMS・リアルタイム更新は作らない。

## 開発手順

- 大きめの実装前にIssueを作成・更新し、目的・責務・依存方向・タスク・受け入れ条件を書く。
- 可能な限りTDDで進め、純粋ロジックは失敗テストを先に置く。仕様変更なしにテストを都合よく変えない。
- 責務・依存方向を変えたら`docs/architecture/`と必要な利用文書を更新する。
- 状態遷移・不変条件は型で表すことを優先し、過剰なrepository pattern・肥大化する`shared`・`unimplemented!()`前提の大きなmodelを避ける。
- GitHub Actionsは原則、利用中actionの最新majorを指定する。

## タスクと運用

rootの[mise.toml](mise.toml)を正とし、`mise tasks ls`で確認して`mise run <task>`を優先する。E2E依存操作もrootの`e2e-*` taskを使う。

- 通常確認: `format`、`test`、`clippy`、`check`、`test-e2e`。
- `dev-local`: private submoduleをremote最新へ同期し、厳格モードのpublish成果物をlocal配信する。同期・publish失敗時はserverを起動しない。
- `sync-obsidian`: 同期のみ。未commit差分があれば停止し、cleanならmerge commitを作らずremote最新をcheckoutする。
- `dev` / `test-e2e-s3`: S3配信の開発確認 / 明示的な実S3 smoke。`OKAWAK_BLOG_ARTIFACT_BUCKET`必須。`dev`はtaskが`OKAWAK_BLOG_ARTIFACT_SOURCE=s3`を設定する。
- `service/`: systemd・Cloudflare Tunnel・運用補助。S3設定・credentials・health/readinessの詳細は[service/README.md](service/README.md)と[service unit](service/okawak_blog.service)を参照する。`sudo`を伴うtaskはVPS運用向け。
