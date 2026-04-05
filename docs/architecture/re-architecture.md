# okawak_blog リアーキテクチャ方針メモ

## 目的

このドキュメントは、`okawak_blog` リポジトリのリアーキテクチャ方針を整理し、実装担当の Codex / AI コーディングエージェントに渡すための設計メモである。

現時点では publisher 側と reader 側の責務分離はまだ移行途中である。ただし publisher 専用の artifact 組み立てとローカル書き出しは `crates/publish/artifacts` へ、Obsidian vault 走査と Markdown 変換は `crates/publish/ingest` へ移し始めており、この方向で `crates/publish/` 側へ責務を寄せていく。そのため、既存コードへの過度な互換性維持よりも、今後の保守性・学習価値・実運用のしやすさを優先して再設計する。

---

## このプロジェクトで本当にやりたいこと

このリポジトリで目指しているのは、一般的なブログサービスや CMS を作ることではない。

やりたいことは以下である。

* 記事執筆は Obsidian で行う
* 記事ソースは別の Obsidian リポジトリで管理する
* Obsidian リポジトリに push した内容を GitHub Actions 経由で処理する
* Rust 製のアプリケーションで公開用成果物を生成する
* 成果物を S3 にアップロードする
* 公開サイトは Leptos ベースで提供する
* サイトは VPS 上で systemd service として単一バイナリで動作させる
* nginx を前段のリバースプロキシとして使う

つまりこれは、**Rust 製のブログ CMS** ではなく、**Rust 製の静的コンテンツ公開基盤 + SSR 表示基盤** である。

---

## 非目標

以下は現時点では作らない、もしくは優先しない。

* DB を使った記事管理
* ユーザー認証・認可
* 管理画面
* UI からの記事作成・編集
* マルチユーザー運用
* 一般的な SaaS ブログサービス機能
* リアルタイム更新
* 複雑なバックオフィス機能

このプロジェクトは 1 人運用を前提としているため、公開専用の設計を優先する。

---

## アーキテクチャの基本方針

### 1. サーバー中心ではなく、コンテンツパイプライン中心にする

今回の主役は常駐 API サーバーではなく、以下のパイプラインである。

1. Obsidian の Markdown を読む
2. Front Matter を解釈・検証する
3. 内部リンクや埋め込みを解決する
4. Markdown を公開用 HTML に変換する
5. 記事一覧やカテゴリ一覧などの index データを生成する
6. 成果物を S3 にアップロードする
7. Leptos SSR サーバーがその成果物を読んで公開する

したがって、アーキテクチャの主軸は「Web サーバー」ではなく「型安全なコンテンツビルドパイプライン」に置く。

### 2. Rust らしい設計を優先する

今後 Rust の実務プロジェクトに入ったとき、このリポジトリの知識がベースになることを意図している。そのため、以下を重視する。

* 純粋なドメインロジックと I/O の分離
* trait による外部境界の抽象化
* 型による不正状態の排除
* 小さく明確な責務分割
* テストしやすい依存方向
* 単一バイナリでのデプロイ容易性

### 3. publisher 側は `crates/publish/`、reader 側は `crates/site/` に寄せる

このリポジトリでは、コード配置そのものも責務境界の一部として扱う。

publisher 側の以下の処理は `crates/publish/` に置く。

* Obsidian vault の読み取り
* Front Matter の解釈・検証
* Markdown → HTML 変換
* 内部リンクや埋め込みの解決
* asset 整形
* S3 へのアップロード

一方で reader 側は `crates/site/` に寄せる。

* `crates/domain`: publisher と reader が共有する純粋な契約とルール
* `crates/site/server`: S3 上の成果物を読むバックエンド
* `crates/site/web`: Leptos SSR UI
* `crates/site/infra`: reader 側の S3 読み取り、キャッシュ、設定読込などの infrastructure

特に `crates/site/infra` は publisher 用の置き場にはしない。Obsidian 読み取り、Markdown renderer、S3 upload をここへ入れないことを明示方針とする。

### 4. SSR は採用する

公開サイトは Leptos SSR で実装する。

ただし、Markdown → HTML の変換責務を「毎リクエスト時の SSR サーバー」に持たせるのではなく、**ビルド時に変換する**方針を取る。

つまり、SSR サーバーは主に以下を担う。

* ルーティング
* レイアウト組み立て
* meta / title / description / canonical 等の埋め込み
* 記事ページ・カテゴリページ・トップページの SSR
* S3 上の成果物の取得

一方で、Markdown の HTML 変換は GitHub Actions 側の公開物生成パイプラインで行う。

---

## SSR 方針の詳細

### 採用方針

* **Leptos SSR は採用する**
* **Markdown → HTML はビルド時に行う**
* **SSR サーバーは S3 上の公開成果物を読む**

### この方針を選ぶ理由

#### 理由 1: ブログ用途では SSR が自然

ブログはトップページ、カテゴリページ、記事ページなどが最初から完成済み HTML で返ると相性がよい。

#### 理由 2: Markdown 処理をブラウザに押し付けない

クライアント側で Markdown を解釈して描画するより、事前に整えた HTML をサーバー側で返す方が安定しやすい。

#### 理由 3: ビルド時変換の方が 1 人運用に向く

現在の運用は push → GitHub Actions → S3 であり、公開成果物生成の中心はすでに CI にある。
そのため Markdown → HTML 変換もそこで行うのが自然である。

#### 理由 4: ランタイムを軽くできる

毎回 S3 から Markdown を取得して変換するより、事前生成済み HTML や index JSON を読むだけの方が本番サーバーを単純にできる。

### 採らない方針

以下は少なくとも初期段階では採らない。

* リクエストごとに Markdown を S3 から取得してその場で HTML 化する設計
* 管理 UI を通した下書きライブプレビュー
* CMS 的な即時編集反映

これらは将来必要になれば検討するが、現時点の要件には過剰である。

---

## 配置・デプロイ方針

### 公開構成

想定する本番構成は以下である。

1. VPS 上で Rust 製サーバーバイナリを systemd service として起動
2. nginx を前段に置いて HTTPS 終端とリバースプロキシを担当
3. Rust サーバーは `127.0.0.1:8008` などで待ち受け
4. Leptos SSR で HTML を返す
5. 記事データや index データは S3 から取得する

### 単一バイナリ運用

アプリケーション本体は単一バイナリで構成する。

この単一バイナリは以下を内包する。

* HTTP サーバー
* ルーティング
* Leptos SSR
* S3 からの成果物取得
* 必要な静的ファイル配信

nginx は別プロセスだが、これは一般的なリバースプロキシであり、アプリケーション本体は単一バイナリとして扱う。

---

## 推奨ワークスペース構成

以下のような workspace 構成を推奨する。

```text
okawak_blog/
├── crates/
│   ├── domain/               # 公開成果物契約と純粋ルール
│   ├── publish/
│   │   ├── publisher/        # publisher CLI エントリポイント
│   │   ├── ingest/           # Obsidian 入力の走査・解析・変換
│   │   ├── artifacts/        # 成果物 bundle と local writer
│   │   └── bookmark/         # bookmark enrichment
│   └── site/
│       ├── infra/            # reader 側の S3 / cache / runtime adapter
│       ├── server/           # 公開成果物を読む統合バックエンド
│       └── web/              # Leptos SSR 公開 UI
├── docs/
│   └── architecture/
└── .github/workflows/
```

### 補足

* `crates/publish/publisher` を publisher 側の中心アプリとして育てる
* `crates/publish/artifacts` は publisher 側の artifact 組み立てと local writer を切り出す補助 crate として扱う
* `crates/publish/ingest` は Obsidian vault 走査、Front Matter 解析、Markdown 変換を切り出す補助 crate として扱う
* `crates/site/infra` を導入または拡張する場合も、reader 側専用として扱う
* `shared` 的な箱は安易に導入しない

---

## 各 crate / app の責務

### `crates/domain`

純粋なドメイン層。

ここには以下のようなものを置く。

* `ArticleMeta`
* `ArticleBody`
* `Article`
* `ArticleSummary`
* `Slug`
* `Category`
* `FrontMatter`
* `PublishableArticle`
* `PublishedArticleSummary`
* `CategoryIndex`
* `SiteMetadata`
* バリデーションルール
* slug 生成ルール
* index 生成に必要な純粋関数

制約:

* I/O を持たない
* S3 を知らない
* Leptos を知らない
* axum を知らない
* async を必須にしない
* 純粋関数と小さな型を中心にする

方針:

* 巨大なエンティティに多責務を持たせすぎない
* `unimplemented!()` が残る設計は避ける
* 型で状態遷移を表現できるなら積極的に使う

### `crates/site/infra`

Leptos サーバーが公開成果物を読むための infrastructure。

ここに以下を置く。

* S3 からの成果物読み取り実装
* 読取キャッシュ
* 設定読込
* reader 側で必要な runtime adapter

ここに置かない。

* Obsidian vault の file system 読み取り
* YAML front matter パーサ実装
* Markdown → HTML 変換実装
* 内部リンク解決実装
* S3 へのアップロード実装
* 画像や asset の処理

想定実装例:

* `S3ArtifactReader`
* `CachedArtifactReader`
* `EnvironmentConfigLoader`

### `crates/site/web`

Leptos SSR の UI 層。

責務:

* トップページ表示
* カテゴリ一覧・記事一覧表示
* 記事詳細ページ表示
* 404
* meta/title/description の設定
* 必要最小限の UI コンポーネント

制約:

* Markdown を直接レンダリングする主役にしない
* 公開用成果物を読む側に徹する
* ドメインモデルと DTO の責務を整理し、不要な二重定義を避ける

### `crates/site/server`

本番用のバックエンド。

役割:

* Axum + Leptos SSR で待ち受ける
* `crates/site/infra` を通して S3 の成果物を取得する
* トップ / カテゴリ / 記事ページを SSR する
* 必要に応じて静的ファイルを配信する

systemd service として起動する単一バイナリの中心はこちらに寄せる。

### `crates/publish/publisher` と publisher 側 crate

CI / ローカル実行用の公開成果物生成 CLI。

役割:

* Obsidian 記事群を読み込む
* 検証する
* HTML を生成する
* index JSON を生成する
* workflow に渡す公開成果物を生成する

必要に応じて ingest / artifacts / bookmark などの publisher 専用 crate を `crates/publish/` 配下で分割する。

### `crates/publish/artifacts`

publisher 側の artifact 組み立てと local writer を担う補助 crate。

役割:

* article index / category index / site metadata の組み立て
* publisher 用 JSON DTO の定義
* `site/` 配下へのローカル書き出し

`publisher` から切り出す publisher 専用ロジックの第一段階の受け皿として扱う。

### `crates/publish/ingest`

publisher 側の Obsidian 入力処理を担う補助 crate。

役割:

* Obsidian vault の走査
* Front Matter の解析
* Markdown body の抽出
* Obsidian link 解決
* Markdown → HTML 変換

`publisher` から scan / parse / render 系の処理を切り出す受け皿として扱う。

### `crates/publish/bookmark`

publisher 側の bookmark enrichment を担う補助 crate。

役割:

* 外部 HTTP を伴う metadata 取得
* OGP / title / favicon の抽出
* bookmark HTML への変換
* 将来的な retry / cache / fail-open の制御

S3 upload は Rust 側へ入れず、GitHub Actions workflow の責務として扱う。

---

## 公開成果物の設計方針

S3 に置く公開成果物の形は、少なくとも次のような構成を想定する。

```text
site/
├── articles/
│   ├── <slug>.html
│   └── index.json
├── categories/
│   ├── tech.json
│   ├── daily.json
│   ├── statistics.json
│   └── physics.json
├── tags/
│   └── index.json
├── assets/
│   └── ...
└── metadata/
    └── site.json
```

### 基本方針

* 記事本文はビルド時に HTML 化しておく
* 一覧やカテゴリページに必要なデータは JSON で保持する
* SSR サーバーはこれらを読み込んで HTML を返す

### メリット

* ランタイムが軽い
* レンダリング差分が減る
* GitHub Actions で公開物検証しやすい
* S3 のオブジェクトがそのままデバッグ対象になる

---

## S3 読み取り方針

初期実装では以下の方針を推奨する。

* 起動時に一覧系の index を読み込むか、あるいは短めのキャッシュを持つ
* 記事本文 HTML は必要時に取得する
* 必要なら ETag / Last-Modified を使って将来的にキャッシュ最適化する

最初から複雑なキャッシュ戦略を入れすぎないこと。
まずはシンプルに実装し、パフォーマンス問題が見えてから最適化する。

---

## テスト方針

### `domain`

* 純粋 unit test
* バリデーション
* slug 生成
* category 解釈
* 記事メタデータ生成

### `crates/site/server` / `crates/site/infra`

* S3 読み取り統合テスト
* キャッシュ戦略の確認
* 設定読込テスト
* SSR 向け読取経路のスモークテスト

### `crates/site/web`

* ルーティングの確認
* SSR 出力のスモークテスト
* 404 やメタタグの確認

### `crates/publish/publisher` / `crates/publish/*`

* front matter parsing test
* Markdown → HTML integration test
* S3 upload の最小限テスト
* E2E に近い統合テスト
* fixture vault → 生成成果物 の確認

---

## Rust らしさとして重視するポイント

このリポジトリは学習目的も兼ねるため、次の観点を意識する。

### 1. 純粋ロジックと I/O の分離

ドメイン・ユースケースと、S3 / filesystem / Leptos / axum などの外部依存を明確に分ける。

### 2. trait による外部境界設計

Java 的に抽象化を濫用するのではなく、必要な入出力だけを薄く trait に切り出す。reader 側の trait は `crates/site/`、publisher 側の trait は `crates/publish/` で閉じることを優先する。

### 3. 型による不正状態の排除

例えば以下のような段階的モデルは有効である。

* `RawFrontMatter`
* `ValidatedFrontMatter`
* `DraftArticleSource`
* `PublishableArticle`

### 4. 単一バイナリでの運用性

複雑な分散構成を目指さず、VPS + systemd + nginx で安定運用しやすい形を優先する。

### 5. 過度な儀式を避ける

Clean Architecture を形式的に真似るのではなく、Rust の実務で役立つ境界設計を目指す。

---

## 採らない設計

以下は現時点では避ける。

### 1. repository パターンの過剰導入

DB がないため、重い repository abstraction は不要。必要なのは filesystem / S3 / renderer などの明確な外部境界である。

### 2. サーバーランタイムに過剰な責務を持たせること

毎回 Markdown をパースして HTML を生成するなど、ビルド時に解決できる責務は CI 側に寄せる。

### 3. shared crate の肥大化

とりあえず何でも shared に入れる構成は避ける。共有が本当に必要な最小限のものだけにする。

### 4. publisher と reader の配置境界を曖昧にすること

publisher 専用の Obsidian / Markdown / upload 実装を `crates/site/` 側へ増やさない。`crates/site/infra` は reader 側専用とする。

### 5. 実装前提の README 先行肥大化

README は理想像の宣言よりも、実際の責務分割と運用方針を正しく反映するようにする。

---

## 移行方針

### Phase 1: `publisher` を主役化する

* `publisher` を公開成果物生成の中心 app として扱う
* markdown 読み取り
* front matter 検証
* HTML 生成
* index JSON 生成
* workflow に渡す公開成果物生成

### Phase 2: publisher 実装を `crates/publish/` 配下で分割する

* ingest / artifacts / bookmark などを必要に応じて `crates/publish/` 配下の crate へ切り出す
* publisher 専用ロジックを `crates/site/` へ増やさない
* `crates/publish/artifacts` と `crates/publish/ingest` を足がかりに publisher 専用ロジックを `crates/publish/` 側へ整理する

### Phase 3: `domain` を小さく本物にする

* 小さな型中心に再設計
* `unimplemented!()` 前提の大きなモデルは分割する
* 純粋テストを増やす

### Phase 4: `site/server` / `site/web` / `site/infra` を reader 側へ絞る

* S3 上の成果物を読む SSR サーバーにする
* 記事生成責務は持たせない
* `crates/site/infra` は reader 側の adapter だけに絞る
* systemd service として動く単一バイナリにまとめる

### Phase 5: README / architecture docs を更新する

* 実装に合った説明へ更新する
* 理想だけでなく、責務とデータフローを明記する

---

## Codex への具体的な期待

Codex には以下を期待する。

1. 現状コードを踏まえて、上記方針に沿う workspace 再編案を提案すること
2. `publisher` を中心に publisher 側を `crates/publish/` 配下で整理する段階的リファクタリング計画を立てること
3. `domain` を小さな型中心に再設計すること
4. publisher と reader の依存境界を崩さずに trait ベースの設計を提案すること
5. `crates/site/server` を単一バイナリの SSR サーバーとして成立させる設計案を示すこと
6. S3 に置く成果物スキーマ案を提案すること
7. 過剰な抽象化を避けつつ Rust 実務で通用する設計に寄せること

---

## 最終的な一文要約

このリポジトリは、**Obsidian で書いた Markdown を `crates/publish/` 配下の Rust 製 publisher が公開成果物へ変換し S3 へ同期可能な形で出力し、それを `crates/site/` 側の単一バイナリ Leptos SSR サーバーが nginx 配下で公開する、Rust らしいコンテンツ公開基盤**として再設計する。
