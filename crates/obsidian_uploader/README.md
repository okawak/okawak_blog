# Obsidian Uploader

ObsidianのMarkdownファイルを解析し、リッチブックマーク付きHTMLファイルを生成するRustアプリケーション。

## 概要

このツールは、Obsidianで作成されたMarkdownファイルを読み取り、以下の処理を行います：

- フロントマター（YAML形式）の解析
- Obsidianリンクの解決とHTML変換
- OGPメタデータを利用したリッチブックマーク生成
- KaTeX数式の処理
- HTMLファイルの生成

## 機能

### コア機能

- **Markdown解析**: pulldown-cmarkを使用した高速なMarkdown処理
- **フロントマター対応**: YAML形式のメタデータ解析
- **リンク解決**: Obsidianの内部リンク（[[記事名]]）を適切なHTMLリンクに変換
- **リッチブックマーク**: HTTPリンクからOGPメタデータを取得し、カード形式で表示
- **数式処理**: KaTeX形式の数式をHTMLクラス付きで出力
- **並列処理**: 複数ファイルの効率的な並列処理（将来実装予定）

### サポートする形式

#### フロントマター
```yaml
---
title: "記事のタイトル"
tags: ["rust", "blog", "tech"]
summary: "記事の概要説明"
priority: 1
created: "2025-01-15T10:00:00+09:00"
updated: "2025-01-15T15:30:00+09:00"
is_completed: true
category: "tech"
---
```

#### リンク形式
- 内部リンク: `[[記事名]]`
- 表示テキスト付きリンク: `[[記事名|表示テキスト]]`
- 通常のMarkdownリンク: `[表示テキスト](URL)`

#### 数式
- インライン数式: `$E = mc^2$`
- ブロック数式: `$$\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$$`

#### リッチブックマーク

通常のHTMLブックマーク要素：
```html
<div class="bookmark">
  <a href="https://example.com">Example Site</a>
</div>
```

上記の形式で記述すると、OGPメタデータを取得して以下のようなリッチブックマークに変換されます：

```html
<div class="bookmark">
  <a href="https://example.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
    <div class="bookmark-container">
      <div class="bookmark-info">
        <div class="bookmark-title">サイトタイトル</div>
        <div class="bookmark-description">サイトの説明文</div>
        <div class="bookmark-link-info">
          <img class="bookmark-favicon" src="https://example.com/favicon.ico" alt="favicon">
          <span class="bookmark-domain">example.com</span>
        </div>
      </div>
      <div class="bookmark-image">
        <img src="https://example.com/ogp-image.jpg" alt="サイトタイトル" loading="lazy">
      </div>
    </div>
  </a>
</div>
```

## 使用方法

### 基本的な使用

```bash
# デフォルト設定で実行
cargo run

# 特定のディレクトリを指定
OBSIDIAN_DIR=/path/to/obsidian OUTPUT_DIR=/path/to/output cargo run
```

### 環境変数

- `OBSIDIAN_DIR`: Obsidianディレクトリのパス（デフォルト: `./obsidian`）
- `OUTPUT_DIR`: 出力ディレクトリのパス（デフォルト: `./dist`）
- `RUST_LOG`: ログレベル（`debug`, `info`, `warn`, `error`）

### 設定例

```bash
# 詳細ログ付きで実行
RUST_LOG=info cargo run

# 本番環境向けリリースビルド
cargo run --release
```

## GitHub Actions連携

AWS S3へのアップロードはGitHub Actionsを使用することを想定しています。
以下のシークレットをGitHubリポジトリに設定してください：

- `AWS_REGION`: AWSリージョン名
- `AWS_ACCOUNT_ID`: AWSアカウントID
- `AWS_ROLE_NAME`: IAMロール名
- `S3_BUCKET`: S3バケット名

## アーキテクチャ

### モジュール構成

```
src/
├── lib.rs              # メインロジックとファイル処理
├── config.rs           # 設定管理
├── error.rs            # エラー型定義
├── models.rs           # データ構造定義
├── parser.rs           # フロントマター解析
├── scanner.rs          # ファイルスキャン
├── converter.rs        # Markdown→HTML変換
├── bookmark.rs         # OGPメタデータ取得とリッチブックマーク生成
└── slug.rs             # URL用スラッグ生成
```

### 処理フロー

1. **スキャン**: 指定ディレクトリ内のMarkdownファイルを検索
2. **解析**: 各ファイルのフロントマターを解析
3. **フィルタリング**: `is_completed: true` のファイルのみを処理対象とする
4. **リンクマッピング**: ファイル間のリンク関係を構築
5. **変換**: Markdown→HTML変換とリッチブックマーク処理
6. **出力**: HTMLファイルの生成

## 開発

### 前提条件

- Rust 1.70以上
- OpenSSL開発ライブラリ（リッチブックマーク機能用）

### セットアップ

```bash
# 依存関係のインストール
cargo build

# テストの実行
cargo test

# 開発用自動リビルド
cargo install cargo-watch
cargo watch -x check -x test
```

### テスト

#### テストの種類

1. **ユニットテスト**: 各モジュールの単体機能
2. **統合テスト**: コンポーネント間の連携
3. **エンドツーエンドテスト**: 実際のObsidianファイルを使用した包括テスト

#### テスト実行

```bash
# 全テスト
cargo test

# 特定のテスト
cargo test test_obsidian_links_conversion

# 詳細ログ付き
RUST_LOG=debug cargo test -- --nocapture

# 統合テストのみ
cargo test --test integration_test

# エンドツーエンドテストのみ
cargo test --test e2e_test
```

### コード品質

```bash
# フォーマット
cargo fmt

# リント
cargo clippy

# 型チェック
cargo check
```

## 設定

### Config構造体

```rust
pub struct Config {
    pub obsidian_dir: PathBuf,
    pub output_dir: PathBuf,
}
```

### 設定の優先順位

1. 環境変数
2. デフォルト値

## エラーハンドリング

### エラーの種類

- `IoError`: ファイル操作エラー
- `YamlError`: フロントマター解析エラー
- `NetworkError`: OGPメタデータ取得エラー
- `PathError`: パス操作エラー

### エラー処理の原則

- 部分的な失敗は処理を継続
- 詳細なエラーログの出力
- 適切なフォールバック機能

## パフォーマンス

### 最適化項目

- String事前確保によるメモリ効率化
- LazyLockによる正規表現の最適化
- 非同期処理によるOGP取得の高速化

### ベンチマーク

```bash
# パフォーマンステスト
cargo test test_large_volume_processing

# ベンチマーク実行（criterion使用時）
cargo bench
```

## ファイル名生成

Obsidianでタイトルを日本語にするとMarkdownのファイル名も日本語になりますが、
扱いやすさのため、タイトルと作成日時からSHA256ハッシュベースの一意な文字列でファイル名を生成します。

## 依存関係

### 主要な外部クレート

- `pulldown-cmark`: Markdown解析
- `serde`, `serde_yaml`: YAML処理
- `reqwest`: HTTP通信
- `scraper`: HTML解析
- `regex`: パターンマッチング
- `tokio`: 非同期ランタイム
- `anyhow`, `thiserror`: エラーハンドリング

### 開発・テスト用

- `tempfile`: 一時ファイル操作
- `rstest`: パラメータ化テスト
- `indoc`: 複数行文字列リテラル

## ライセンス

MIT License

## コントリビューション

1. Issueを作成して問題を報告
2. フォークしてフィーチャーブランチを作成
3. テスト駆動開発でコードを実装
4. プルリクエストを作成

### 開発ガイドライン

- テストファーストの開発
- コードレビューの実施
- ドキュメントの更新
- パフォーマンステストの実行
