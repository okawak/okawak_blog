[![Sync Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

## アーキテクチャ

このプロジェクトは、従来のクリーンアーキテクチャではなく、**Rust-firstアーキテクチャ**を採用しています。これは、Rustの所有権システムとエコシステムに最適化された設計哲学です。

### なぜクリーンアーキテクチャを採用しないのか

1. **trait抽象化の過度な使用**: Rustの所有権システムでは、過度なtrait抽象化がコンパイル時間の増大とコードの複雑化を招く
2. **OOP的な依存性逆転**: 関数型とゼロコスト抽象化を重視するRustの哲学と相反する
3. **Leptos Server Functionsとの不整合**: モダンなRust Webエコシステムでは、Server Functionsが推奨パターン
4. **所有権モデルとの衝突**: 複雑な抽象化層は借用チェッカーとの戦いを生む

### Rust-firstアーキテクチャの原則

#### 1. 具象実装優先
```rust
// ❌ 過度な抽象化
trait BlogRepository {
    async fn get_post(&self, id: &str) -> Result<BlogPost>;
}

// ✅ 具象実装
pub struct BlogService {
    s3: S3Service,
    config: ServiceConfig,
}
```

#### 2. Leptos Server Functions活用
```rust
// ✅ Web層との統合にServer Functionsを使用
#[server(GetBlogPost, "/api")]
pub async fn get_blog_post(id: String) -> Result<BlogPost, ServerFnError> {
    // サービス層の具象実装を直接呼び出し
}
```

#### 3. 所有権フレンドリーな設計
- 不要な`Arc<dyn Trait>`を避ける
- `Clone`可能な軽量構造体を優先
- 借用チェッカーと協調する設計

### クレート構成

```
web → service → domain
 ↓       ↓       ↓
 UI   ビジネス  コア
     ロジック   ドメイン
```

#### `web` クレート
- **責務**: フロントエンド（Leptos + thaw-ui）
- **依存**: `service`のみ
- **特徴**: Server Functionsでサービス層と統合

#### `service` クレート
- **責務**: ビジネスロジック、外部システム統合
- **依存**: `domain`, AWS SDK, 具象実装
- **特徴**: Rust生態系のベストプラクティスに従った具象実装

#### `domain` クレート
- **責務**: コアビジネスルール、エンティティ
- **依存**: 最小限（serde, chrono等）
- **特徴**: pure Rust、trait最小限

### 利点

1. **コンパイル時間の短縮**: 複雑なtrait解決が不要
2. **明確な依存関係**: 具象実装により依存が明確
3. **Rustエコシステムとの親和性**: Server Functions等モダンパターンの活用
4. **保守性**: 抽象化層の削減により理解しやすいコード

## 利用可能なタスク

### 開発環境
```bash
# 開発サーバー起動（CSSも含む）
mise dev

# CSS生成のみ
mise css

# コードフォーマット
mise format
```

### VPS デプロイ
```bash
# 一括デプロイ
mise deploy

# 個別実行
mise stop        # サービス停止
mise build       # プロダクションビルド
mise start       # サービス開始
```

### 運用・監視
```bash
mise status      # サービス状態確認
mise logs        # リアルタイムログ
mise logs-recent # 最新ログ
mise restart     # サービス再起動
```

### ユーティリティ
```bash
mise check-deps  # 依存関係チェック
```

## 準備

### 必要なツール
```bash
# Rust インストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# mise インストール (タスクランナー)
curl https://mise.run | sh

# cargo-leptos インストール
cargo install cargo-leptos

# stylance CLI インストール
cargo install stylance-cli

# AWS CLI インストール (オプション)
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awsclip.zip"
unzip awsclip.zip
sudo ./aws/install
```

## デプロイ手順

### 1. リポジトリクローン
```bash
cd /home/okawak
git clone <repository-url> okawak_blog
cd okawak_blog
```

### 2. 依存関係チェック
```bash
# 必要なツールがインストールされているかチェック
mise check-deps
```

### 3. AWS設定 (必要な場合)
```bash
aws configure --profile blog-s3
# または ~/.aws/config と ~/.aws/credentials を手動設定
```

### 4. 自動デプロイ実行
```bash
# 一括デプロイ（CSS生成→ビルド→サービス設定→起動）
mise deploy
```

### 5. 個別コマンド実行 (詳細制御が必要な場合)

#### CSS生成
```bash
mise css
```

#### ビルド
```bash
mise build
```

#### サービス設定とデプロイ
```bash
# サービス停止→設定コピー→ビルド→起動
mise stop
mise service
mise start
```

## 運用コマンド

### サービス状態確認
```bash
mise status
```

### ログ確認
```bash
# リアルタイムログ
mise logs

# 最新のログ
mise logs-recent
```

### サービス再起動
```bash
mise restart
```

### サービス停止
```bash
mise stop
```

### 開発サーバー起動
```bash
mise dev
```

## 設定ファイル

### systemd サービス設定
- パス: `/etc/systemd/system/okawak_blog.service`
- 設定内容: セキュリティ強化、リソース制限、環境変数

### Leptos設定
- パス: `crates/web/Cargo.toml` の `[package.metadata.leptos]`
- 本番環境: `env = "PROD"`, `site-addr = "0.0.0.0:8008"`

### 環境変数
- 開発用: デフォルト設定で動作
- 本番用: systemdサービス内で設定（キーは含まず、AWS CLIで別途設定）

## トラブルシューティング

### サービスが起動しない
```bash
# エラー詳細確認
mise status
mise logs-recent

# 権限確認
ls -la /home/okawak/okawak_blog/
sudo chown -R okawak:okawak /home/okawak/okawak_blog/
```

### ビルドエラー
```bash
# 依存関係確認
mise check-deps
mise clean
mise build

# CSS生成エラー
mise css
```

### ポート確認
```bash
# ポート8008の使用状況
sudo netstat -tlnp | grep :8008
sudo ss -tlnp | grep :8008
```

### ファイアウォール設定
```bash
# ufw の場合
sudo ufw allow 8008

# firewalld の場合
sudo firewall-cmd --permanent --add-port=8008/tcp
sudo firewall-cmd --reload
```

## セキュリティ設定

systemdサービスには以下のセキュリティ機能が有効化されています：

- `NoNewPrivileges=true` - 権限昇格防止
- `ProtectSystem=strict` - システムディレクトリ保護
- `ProtectHome=true` - ホームディレクトリ保護
- `PrivateTmp=true` - 一時ディレクトリ分離
- `RestrictNamespaces=true` - ネームスペース制限

## 監視とメンテナンス

### ヘルスチェック
```bash
# サービス状態
curl -I http://localhost:8008

# メモリ使用量
systemctl show okawak_blog --property=MemoryCurrent
```

### ログローテーション
journaldが自動でログローテーションを行います。

### バックアップ
```bash
# アプリケーションバックアップ
tar -czf okawak_blog_backup_$(date +%Y%m%d).tar.gz /home/okawak/okawak_blog/
```

## 更新手順

1. コード更新 (`git pull`)
2. 依存関係チェック (`mise check-deps`)
3. デプロイ実行 (`mise deploy`)
4. 動作確認 (`mise status`)
