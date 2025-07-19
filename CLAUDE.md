# CLAUDE.md

## Project Information

このプロジェクトは、RustのLeptosフレームワークを用いてブログを構築することです。
以下のガイドラインに従って開発を進めてください。

## Conversation Guidelines

- 常に日本語で会話する

## Architecture Guidelines

- デプロイ先のインフラは、`./terraform`ディレクトリの中で定義されている。
- インフラ構築には、Terraformが使用されています。
    - このディレクトリの中では、**絶対にコマンドを実行しないでください**。
    - また、このディレクトリの中のファイルについて**編集することは禁止です**。
    - ファイルの読み取りのみ可能で、どのような機能があるかを確認してください。
- ブログの構築には二段階の処理が行われています。
- 一段階目は、NotionもしくはObsidianのファイルを読み込んで、それをgithub actionsを用いて、AWS S3にアップロードします。
    - 現状はNotionを使用しています(`crates/notion_api`)が、将来的にObsidianに移行する(`crates/obsidian_uploader`)予定です。
    - Obsidianはプライベートリポジトリで、git submoduleとして管理されています。
    - 開発途中ですが、Obsidianのmarkdownファイルを読み込んで、適切な形のHTMLに変換して、S3にアップロードする機能をこれから実装していきます。
    - 現状は、NotionのAPIを使用して、S3にはmarkdownファイルがアップロードされており、この構造も変更する予定です。
- 二段階目は、S3にアップロードされたHTMLファイルを、Leptosフレームワークを用いて、ブログとして表示する機能を実装します。
    - 詳細な設計は未定で、これから議論して決めていきたいですが、現状はLeptosのフレームワーク内で、S3からファイルを読み取って公開する設定にしています。
    - Leptosのフレームワークは、`crates/leptos_blog`に実装されています。
    - SSRの設定で、Leotosを使用します。
    - Leptosのフレームワーク部分とバックエンドの処理を分けて実装すべきかは、ベストプラクティスに基づいて議論して決めていきます。

## Build and Run Guidelines

- 各プロジェクトのビルドはCargoを使用します。
    ```bash
    cargo build --release -p <crate_name>
    ```
- Leptosフレームワークのビルドは`cargo leptos`を使用します。
    ```bash
    cargo leptos build --release
    ```

## Development Philosophy

### Test-Driven Development (TDD)

- 原則としてテスト駆動開発（TDD）で進める
- 期待される入出力に基づき、まずユニットテストを作成する
- 実装コードは書かず、テストのみを用意する
- テストを実行し、失敗を確認する
- テストが正しいことを確認できた段階でコミットする
- その後、テストをパスさせる実装を進める
- 実装中はテストを変更せず、コードを修正し続ける
- すべてのテストが通過するまで繰り返す

### Coding Standards

- コードはRustの公式スタイルガイドに従う
- 変数名、関数名は意味のある名前を付ける
- コメントは必要な箇所にのみ記述し、コードの意図を明確にする
- Rustのドキュメンテーションコメント（///）を使用して、関数や構造体の説明を記述する

### Workflow

- GitHub Flowを採用
    - ブランチは機能ごとに分ける
    - プルリクエストを作成し、コードレビューを受ける
    - レビュー後、マージしてmainブランチに統合する
    - レビュー、マージはClaude Codeで**勝手に行わない**。

## Development Progress

### Phase 3: Obsidian Uploader リッチブックマーク機能開発

#### 完了した作業（2025-01-13）

**1. Phase 3の要件定義**
- Obsidian記事内のHTTPリンクを自動検出し、OGPメタデータを取得
- リッチブックマーク形式のHTMLを生成する機能
- TDDアプローチでの実装

**2. TDDによるテスト先行開発**
- `bookmark.rs`モジュールに包括的なテストスイート作成
- OGPメタデータ取得機能のテスト
- HTMLテンプレート生成機能のテスト
- エラーハンドリングとフォールバック機能のテスト
- 全57テストケースで完全にパス

**3. 実装した機能**

**A. bookmark.rsモジュール**
- `BookmarkData`構造体：OGPメタデータ（URL、タイトル、説明、画像、ファビコン）を保持
- `fetch_ogp_metadata()`：非同期でWebページからOGPメタデータを取得
- `generate_rich_bookmark()`：BookmarkDataからリッチブックマークHTMLを生成
- `convert_simple_bookmarks_to_rich()`：HTML内のシンプルブックマークを検出してリッチ化
- `create_fallback_bookmark_data()`：メタデータ取得失敗時のフォールバック

**B. 依存関係**
- `reqwest`：HTTP通信とOGPメタデータ取得
- `scraper`：HTML解析とメタタグ抽出
- `url`：URL解析と相対パス解決
- `regex`：ブックマーク構造の検出

**C. HTMLテンプレート**
- `notion-bookmark`から`bookmark`クラスへの統一
- レスポンシブ対応とアクセシビリティ配慮
- 画像遅延読み込み（loading="lazy"）対応
- target="_blank"とrel="noopener noreferrer"でセキュリティ確保

**4. コードレビューフィードバック対応**
- converter.rsから970行→540行に削減（bookmark機能の分離）
- bookmark関連テストの適切なモジュール配置
- 不要なpub use文の整理とモジュール境界の明確化
- コンパイル警告の完全解消

**5. 現在の状態**
- ブランチ：`feat_obsidian_uploader`
- 全テスト通過：57/57 tests passing
- 警告・エラー：0件
- 実装完了度：100%

**6. 技術的特徴**
- 非同期処理によるパフォーマンス最適化
- エラー時の適切なフォールバック機能
- HTMLエスケープによるXSS対策
- モジュール分離による保守性向上
- 包括的なテストカバレッジ

**7. 次のステップ（検討事項）**
- Obsidian記事の実際のmarkdownファイルでの動作テスト
- S3アップロード機能との統合テスト
- パフォーマンスベンチマークの実施
- エラーログとモニタリングの強化
