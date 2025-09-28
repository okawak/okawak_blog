# AGENTS.md

## プロジェクト情報

このプロジェクトは、RustのLeptosフレームワークを用いてブログを構築することです。
以下のガイドラインに従って開発を進めてください。

## 会話ガイドライン

- 常に日本語で会話する

## プロジェクト全体像

### システム概要
- **プロジェクト名**: okawak_blog - 個人ブログシステム（https://www.okawak.net）
- **アーキテクチャ**: Rust-First Architecture（Clean Architectureではない）
- **技術スタック**: Rust, Leptos, Axum, AWS S3, thaw-ui, stylance
- **デプロイ**: 単一バイナリによる統合デプロイ（~12MB）

### ワークスペース構成

#### メインクレート (`crates/`)
```
okawak_blog/
├── domain/           # 純粋ドメインロジック
├── server/           # 統合バックエンド  
├── web/              # Leptosフロントエンド
└── apps/             # 補助アプリケーション
    └── obsidian_uploader/
```

- **`domain`**: 純粋ドメインロジック（I/Oなし、同期のみ、WASM対応）
- **`server`**: 統合バックエンド（Axum + Leptos統合、AWS S3連携）
- **`web`**: Leptosフロントエンド（SSR + CSR、thaw-ui、stylance CSS-in-Rust）

#### アプリケーション (`apps/`)
- **`obsidian_uploader`**: Obsidianファイル→S3アップローダー
  - Obsidianはプライベートリポジトリでgit submoduleとして管理
  - markdownファイルを適切なHTMLに変換してS3にアップロード

#### インフラ (`terraform/`)
- AWS S3、OCI等のインフラ定義
- **注意**: このディレクトリでは**絶対にコマンドを実行しない**
- **編集禁止**: ファイルの読み取りのみ可能

#### 運用 (`service/`)
- systemdサービス設定
- nginx設定  
- 自動デプロイスクリプト

### データフロー
```
Stage 1: Obsidianファイル → obsidian_uploader (GitHub Actions) → AWS S3
Stage 2: Browser ←→ Leptos SSR ←→ Server Functions ←→ UseCases ←→ Domain Logic
                           ↓                    ↓
                      HTTP Handlers      Infrastructure (S3)
                           ↓                    ↓
                       Axum Router          AWS S3
```

## アーキテクチャガイドライン

### Rust-First Architecture の原則

#### 1. ドメイン純粋性の確保
```rust
// ✅ 純粋関数によるビジネスロジック（I/Oなし、同期のみ）
pub fn generate_slug_from_title(title: &Title) -> Result<Slug> {
    // 副作用なし、テスト容易、WASM対応
}
```

#### 2. 統合サーバー設計
```rust
// ✅ 単一バイナリによる統合デプロイ
pub fn create_app(repository: Arc<Repository>, storage: Arc<Storage>) -> Router {
    // Axum + Leptos統合、型安全な依存注入
}
```

#### 3. 型駆動開発
```rust
// ✅ 不正な状態をコンパイル時に防ぐ
pub struct PublishedArticle(Article);  // 公開済み記事のみ
impl From<DraftArticle> for PublishedArticle { /* ... */ }
```

### アーキテクチャ決定記録（ADR）
重要な設計決定は`docs/adr/`ディレクトリで文書化されています：
- [ADR-0001: Rust-First アーキテクチャの採用](./docs/adr/0001-rust-first-architecture.md)
- [ADR-0002: ドメイン層の純粋化](./docs/adr/0002-domain-layer-purification.md)
- [ADR-0003: サーバー層統合設計](./docs/adr/0003-server-layer-integration.md)

### なぜClean Architectureではないのか
1. **WASM互換性**: tokioの`net`機能はWASMで利用不可
2. **型安全性**: Rustの型システムによるコンパイル時制約
3. **パフォーマンス**: ゼロコスト抽象化の最大活用
4. **運用シンプル**: 単一バイナリによるデプロイ簡素化

## ビルドと実行ガイドライン

### cargo-makeによる統合ビルドシステム

#### 開発環境
```bash
# Leptos開発サーバー起動
cargo make dev

# 統合開発環境（Server + Leptos）
cargo make integrated-dev

# ファイル変更を監視して自動リビルド
cargo make watch

# コードフォーマット
cargo make format
```

#### ビルド & デプロイ
```bash
# 統合ビルド（Leptos + Server）
cargo make build

# Leptosフロントエンドのみ
cargo make build-web

# サーバーバイナリのみ
cargo make build-server

# 完全デプロイフロー
cargo make full-deploy

# 本番環境デプロイ（nginx含む）
cargo make production-deploy
```

#### テスト & 品質保証
```bash
# 全テスト実行
cargo make test

# ドメイン層テスト（純粋）
cargo make test-domain

# サーバー統合テスト
cargo make test-server

# Webフロントエンドテスト
cargo make test-web

# コード解析
cargo make clippy

# 高速シンタックスチェック
cargo make check
```

### 各プロジェクトのビルド
- 通常のCargoビルド: `cargo build --release -p <package_name>`
- Leptosフレームワークのビルド: `cargo leptos build --release`

## 開発哲学

### Test-Driven Development (TDD)

- 原則としてテスト駆動開発（TDD）で進める
- 期待される入出力に基づき、まずユニットテストを作成する
- 実装コードは書かず、テストのみを用意する
- テストを実行し、失敗を確認する
- テストが正しいことを確認できた段階でコミットする
- その後、テストをパスさせる実装を進める
- 実装中はテストを変更せず、コードを修正し続ける
- すべてのテストが通過するまで繰り返す

### コーディング標準

- コードはRustの公式スタイルガイドに従う
- 変数名、関数名は意味のある名前を付ける
- コメントは必要な箇所にのみ記述し、コードの意図を明確にする
- Rustのドキュメンテーションコメント（///）を使用して、関数や構造体の説明を記述する

### ワークフロー

- GitHub Flowを採用
  - ブランチは機能ごとに分ける
  - プルリクエストを作成し、コードレビューを受ける
  - レビュー後、マージしてmainブランチに統合する
  - **注意**: レビュー、マージはAIエージェントで**勝手に行わない**

## 技術詳細

### パフォーマンス特性
- **ビルドサイズ**: 
  - Server バイナリ: ~12MB (release build)
  - WASM: ~2MB (gzip済み)
- **実行時性能**:
  - 起動時間: <100ms
  - メモリ使用量: ~50MB (base)
  - レスポンス時間: <50ms (static content)

### 型安全性とWASM互換性
```rust
// コンパイル時制約の例
struct DraftArticle { /* ... */ }
struct PublishedArticle { /* ... */ }

// 公開記事のみを返すAPI
fn get_published_articles() -> Vec<PublishedArticle> {
    // DraftArticleは返却不可（コンパイルエラー）
}

// domain層は完全にWASM対応
#[cfg(target_arch = "wasm32")]
fn browser_side_validation(article: &Article) -> bool {
    domain::validate_article_data(article).is_ok()
}
```

### 依存関係管理
- **domain**: 最小限（thiserror のみ）
- **server**: AWS SDK、Axum、Leptos統合
- **web**: Leptos、thaw-ui、stylance、型安全なCSS-in-Rust

## 重要な制約事項

### 必須制約
1. **ドメイン層は純粋関数のみ**: I/O操作禁止、同期のみ、WASM対応必須
2. **terraformディレクトリは編集禁止**: 読み取りのみ可能
3. **Obsidianはgit submodule**: プライベートリポジトリとして管理
4. **GitHub Actions**: Obsidianファイルの自動S3アップロード

### 開発プロセス

#### 実装前の準備
- **実装方針ドキュメント作成必須**: 具体的な実装に入る前に、必ず`docs/implementation-plans/`に実装方針ドキュメントを作成する
- **ドキュメント内容**: 実装方針・アーキテクチャ設計・各層の責任のみ記載し、具体的なコードは記載しない
- **GitHub Issue作成**: 大きな機能追加の場合は、GitHub issueで実装計画を明文化してから開始
- **レビュー・確認**: 実装方針が確定してから具体的な実装を開始する

#### SSR中心の実装方針
- **Web層**: Server-Side Rendering (SSR) を中心とした実装
- **CSR移行**: パフォーマンス上の利点が明確な場合のみ、CSR + SSRのハイブリッドに移行を検討
- **初期実装**: まずはSSRでシンプルに実装し、必要に応じて段階的にクライアント機能を追加

### 開発時の注意点
- terraform/ディレクトリでコマンド実行しない
- ドメイン層でasync/await使用しない
- 型安全性を最優先に設計する
- ADRで重要な設計決定を文書化する
- **実装方針ドキュメント優先**: 具体的なコード実装前に必ず実装方針を文書化
- **TDD遵守**: テスト駆動開発で進める（テスト→実装→リファクタリング）

## セットアップ

### 必要なツール
```bash
# Rust インストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# cargo-make インストール（タスクランナー）
cargo install cargo-make

# cargo-leptos インストール
cargo install cargo-leptos

# stylance CLI インストール（CSS-in-Rust）
cargo install stylance-cli
```

### 依存関係チェック
```bash
cargo make check-deps
```

## トラブルシューティング

### ビルドエラー
```bash
# WASM関連エラー
cargo make check-domain    # ドメイン層の純粋性確認

# 依存関係エラー
cargo make clean
cargo make check-deps
cargo make build
```

### サービス起動エラー
```bash
# ログ確認
cargo make logs-recent

# ポート確認
sudo netstat -tlnp | grep :8008

# 権限確認
sudo chown -R okawak:okawak /home/okawak/okawak_blog/
```

このガイドラインに従って、Rust-First Architectureの原則を守りながら開発を進めてください。