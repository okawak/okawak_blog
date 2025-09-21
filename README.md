[![Sync Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

## アーキテクチャ

このプロジェクトは、従来のClean Architectureではなく、**Rust-First Architecture**を採用しています。これは、Rustの所有権システムとゼロコスト抽象化を最大限活用した設計哲学です。

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

### クレート構成

```
okawak_blog/
├── domain/           # 純粋ドメインロジック
├── server/           # 統合バックエンド
├── web/              # Leptosフロントエンド
└── apps/             # 補助アプリケーション
    └── obsidian_uploader/
```

#### `domain` クレート - 純粋ドメイン層
- **責務**: ビジネスルール、エンティティ、純粋関数
- **特徴**: I/O操作なし、同期のみ、WASM対応
- **依存**: 最小限（serde, chrono, thiserror, uuid）

```rust
// domain/src/business_rules.rs
pub fn validate_article_content(content: &str) -> Result<(), DomainError> {
    // 純粋関数によるバリデーション
}
```

#### `server` クレート - 統合バックエンド
- **責務**: usecases, ports, infrastructure, handlers
- **特徴**: 単一バイナリ、Axum + Leptos統合、AWS S3連携
- **依存**: domain, aws-sdk, axum, leptos-server

```rust
// server/src/main.rs - 統合エントリーポイント
#[tokio::main]
async fn main() -> Result<()> {
    let app = create_app(repository, storage);
    axum::serve(listener, app).await
}
```

#### `web` クレート - Leptosフロントエンド
- **責務**: SSR + CSR、UI コンポーネント、styling
- **特徴**: thaw-ui、stylance CSS-in-Rust、server functions
- **依存**: domain（ドメインロジック共有）, leptos, thaw

### アーキテクチャ決定記録（ADR）

重要なアーキテクチャ決定は [docs/adr/](./docs/adr/) で文書化されています：

- [ADR-0001: Rust-First アーキテクチャの採用](./docs/adr/0001-rust-first-architecture.md)
- [ADR-0002: ドメイン層の純粋化](./docs/adr/0002-domain-layer-purification.md)
- [ADR-0003: サーバー層統合設計](./docs/adr/0003-server-layer-integration.md)

### なぜClean Architectureではないのか

1. **WASM互換性**: tokioの`net`機能はWASMで利用不可
2. **型安全性**: Rustの型システムによるコンパイル時制約
3. **パフォーマンス**: ゼロコスト抽象化の最大活用
4. **運用シンプル**: 単一バイナリによるデプロイ簡素化

## 利用可能なタスク

cargo-makeによる統合ビルドシステムを提供しています。

### 開発環境

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

### ビルド & デプロイ

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

### テスト & 品質保証

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

### 運用・監視

```bash
cargo make status      # サービス状態確認
cargo make logs        # リアルタイムログ
cargo make logs-recent # 最新ログ
cargo make restart     # サービス再起動
```

### ヘルプ

```bash
cargo make help        # 利用可能なタスク一覧
cargo make check-deps  # 依存関係チェック
```

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

## デプロイメント

### 単一バイナリデプロイ

```bash
# 1. リポジトリクローン
git clone <repository-url> okawak_blog
cd okawak_blog

# 2. 依存関係チェック
cargo make check-deps

# 3. 統合ビルド & デプロイ
cargo make full-deploy
```

生成される成果物：
- `/target/release/server` - 統合サーバーバイナリ（12MB）
- システムサービスとして動作（systemd）

### 設定管理

#### systemd サービス設定
- パス: `/etc/systemd/system/okawak_blog.service`
- セキュリティ: NoNewPrivileges, ProtectSystem等の強化設定

#### 環境変数
- 開発: デフォルト設定で動作
- 本番: systemdサービス内で設定（AWS認証情報は別途）

## パフォーマンス特性

### ビルドサイズ
- Server バイナリ: ~12MB (release build)
- WASM: ~2MB (gzip済み)

### 実行時性能
- 起動時間: <100ms
- メモリ使用量: ~50MB (base)
- レスポンス時間: <50ms (static content)

## 開発ワークフロー

### 1. ローカル開発
```bash
cargo make dev-flow        # Leptos開発サーバー
cargo make integrated-dev  # Full stack開発
```

### 2. テスト & 品質チェック
```bash
cargo make test-domain     # ドメインロジック検証
cargo make check-server    # サーバー型チェック
cargo make clippy          # コード品質確認
```

### 3. デプロイ
```bash
cargo make quick-deploy    # 高速デプロイ
cargo make production-deploy # 本番デプロイ
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

## アーキテクチャ詳細

### データフロー

```
Browser ←→ Leptos SSR ←→ Server Functions ←→ UseCases ←→ Domain Logic
                           ↓                    ↓
                      HTTP Handlers      Infrastructure
                           ↓                    ↓
                       Axum Router          AWS S3
```

### 型安全性

```rust
// コンパイル時制約の例
struct DraftArticle { /* ... */ }
struct PublishedArticle { /* ... */ }

// 公開記事のみを返すAPI
fn get_published_articles() -> Vec<PublishedArticle> {
    // DraftArticleは返却不可（コンパイルエラー）
}
```

### WASM互換性

```rust
// domain層は完全にWASM対応
#[cfg(target_arch = "wasm32")]
fn browser_side_validation(article: &Article) -> bool {
    domain::validate_article_data(article).is_ok()
}
```
