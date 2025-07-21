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

# 本番環境向けリリースビルド
cargo run --release
```

### ディレクトリ構成

- 入力ディレクトリ: `./obsidian` (固定)
- 出力ディレクトリ: `./dist` (固定)

## GitHub Actions連携

AWS S3へのアップロードはGitHub Actionsを使用します。
Obsidianファイルはプライベートリポジトリで管理されており、GitHub Appを使用してアクセスしています。

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

### セットアップ

```bash
# 依存関係のインストール
cargo build

# テストの実行
cargo test
```

### テスト

```bash
# 全テスト実行
cargo test

# 統合テストのみ
cargo test --test integration_test

# E2Eテストのみ
cargo test --test e2e_test
```
