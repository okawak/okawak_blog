[![Publish Content to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

Obsidianで執筆したMarkdownを日英の公開コンテンツへ変換し、Topcoat SSRで配信する個人ブログ基盤です。公開処理はビルド時に完了させ、VPSのserverはS3上のartifactを読んでrouting、metadata、UIを組み立てます。

## データフロー

```text
private Obsidian（obsidian/ submodule）
  -> export: 公開対象の抽出・正規化・翻訳
  -> content/: review済みの日英Markdownとタグ辞書
  -> publish: HTML・JSON artifactの生成
  -> GitHub Actions: immutable S3 releaseの検証と公開
  -> Topcoat SSR server
  -> Cloudflare Tunnel
  -> Browser
```

private Obsidianが日本語原文の正本です。`export`は公開対象だけを`content/`へ書き出し、翻訳の手動編集を保護します。GitHub ActionsはGit管理済みの公開コンテンツだけを使うため、private submoduleやAI認証を必要としません。

## リポジトリ構成

```text
okawak_blog/
├── obsidian/          # private Obsidian submodule
├── content/           # Git管理する公開Markdown・タグ辞書
├── translation.json   # 記事・タグ・UI共通の翻訳設定と用語集
├── crates/
│   ├── export/        # private入力の抽出・正規化・翻訳
│   ├── domain/        # 公開コンテンツ・artifact・pageの共有契約
│   ├── publish/       # 公開Markdownからartifactを生成
│   ├── infra/         # local / S3 artifact reader
│   └── server/        # Topcoat application、UI、HTTP runtime
├── e2e/               # browser E2Eと固定artifact
├── docs/              # architecture、content契約、runbook
├── scripts/           # 検証・運用スクリプト
├── service/           # systemd unitとruntime運用
└── terraform/         # cloud resource定義
```

依存方向、各crateの責務、artifact・URL・runtimeの不変条件は[architecture](./docs/architecture/architecture.md)を正本とします。

## ローカル開発

タスク定義は[mise.toml](./mise.toml)にあります。利用可能なtaskは`mise tasks ls`で確認できます。

```bash
mise install

# private原文をremote最新へ同期する
mise run sync-obsidian

# 公開対象を抽出・翻訳する
mise run export

# Git管理する公開Markdownからartifactを作り、local serverを起動する
mise run dev-local
```

`sync-obsidian`は現在または移動前のsubmoduleに未処理の作業がある可能性を検出すると停止します。翻訳候補の確認・手動修正・採用方法は[export README](./crates/export/README.md)を参照してください。

通常の確認は次のtaskを使います。

```bash
mise run format
mise run test
mise run clippy
mise run check
mise run test-e2e
```

実S3を使う開発serverとsmoke testは[e2e README](./e2e/README.md)に従ってください。

## 公開と運用

公開Markdownと辞書をreviewしてmainへ統合した後、GitHub Actionsの`Publish Content to S3`を手動実行します。workflowは対象commitのCIと最新mainを確認し、releaseをS3へuploadしてbrowser E2Eを通した後だけ`current.json`を切り替えます。

server binaryとUI assetの配備はコンテンツ公開と分離しています。通常は管理端末から`mise run deploy-vps`を実行します。VPS、AWS認証、Cloudflare Tunnelの設定と障害対応は[operations](./docs/operations/README.md)を参照してください。

## 文書

- [architecture](./docs/architecture/architecture.md): 現行の責務、依存方向、重要な契約と不変条件
- [Obsidian template](./docs/content/obsidian-template.md): private入力のfrontmatterと執筆規則
- [公開Markdown契約](./docs/content/public-markdown.md): `export`と`publish`の境界schema
- [export](./crates/export/README.md): 抽出・翻訳・候補採用の操作と保証
- [publish](./crates/publish/README.md): artifact生成の入力、出力、検証
- [server](./crates/server/README.md): Topcoat applicationとUIの実装境界
- [UI文言カタログ](./crates/server/locales/README.md): server辞書と翻訳の運用
- [browser E2E](./e2e/README.md): 固定fixtureと実S3の確認
- [runtime service](./service/README.md): systemd、reader設定、probe、cache
- [operations](./docs/operations/README.md): 本番の構築・更新・障害対応runbook
- GitHub Issues / PRs: 実装計画、移行経緯、完了済み作業の記録

DB記事管理、認証・認可、管理画面、ブラウザ編集、マルチユーザー、リアルタイム更新は対象外です。
