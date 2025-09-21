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
- 一段階目は、Obsidianのファイルを読み込んで、それをgithub actionsを用いて、AWS S3にアップロードします。
    - `apps/obsidian_uploader`クレートで作成されています。
    - Obsidianはプライベートリポジトリで、git submoduleとして管理されています。
    - Obsidianのmarkdownファイルを読み込んで、適切な形のHTMLに変換して、S3にアップロードします。
- 二段階目は、S3にアップロードされたHTMLファイルを、Leptosフレームワークを用いて、ブログとして表示する機能を実装します。
    - SSRの設定で、Leotosを使用します。
    - README.mdに記載したアーキテクチャで進めていきます。

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

### ADR (Architecture Decision Records)

- 重要な設計決定はADRとして記録する
- ADRは`docs/adr`ディレクトリに保存する

### Workflow

- GitHub Flowを採用
    - ブランチは機能ごとに分ける
    - プルリクエストを作成し、コードレビューを受ける
    - レビュー後、マージしてmainブランチに統合する
    - レビュー、マージはClaude Codeで**勝手に行わない**。
