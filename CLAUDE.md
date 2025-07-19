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
