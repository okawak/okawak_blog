[![Sync Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/upload.yml) [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)

# ぶくせんの探窟メモ

https://www.okawak.net

`okawak_blog` は、Obsidian で書いた Markdown を Rust 製の publisher が公開成果物へ変換して S3 に配置し、それを VPS 上の単一バイナリ Leptos SSR サーバーが nginx 配下で公開する、静的コンテンツ公開基盤 + SSR 表示基盤です。

## 関連文書

- [docs/architecture/re-architecture.md](./docs/architecture/re-architecture.md): 目標アーキテクチャへ移行するための設計メモ
- [docs/adr/](./docs/adr/): 設計判断の記録
- GitHub Issues / PRs: 実装計画、進捗、作業単位の管理

## このリポジトリが担うこと

- 記事は Obsidian で執筆する
- 記事ソースは別の Obsidian リポジトリで管理する
- GitHub Actions またはローカル実行の publisher が公開成果物を生成する
- 生成した HTML / index JSON / assets を S3 に配置する
- Leptos SSR サーバーが S3 上の成果物を読んで配信する
- VPS + `systemd` + `nginx` で単純に運用できる構成を保つ

## これは何ではないか

このプロジェクトは、一般的なブログ CMS や SaaS ブログサービスを作るものではありません。

現時点での非目標は以下です。

- DB を使った記事管理
- ユーザー認証・認可
- 管理画面
- ブラウザ UI からの記事作成・編集
- マルチユーザー運用
- 複雑なバックオフィス機能

## 目指すアーキテクチャ

### コンテンツパイプライン中心

主役は常駐 API サーバーではなく、公開成果物生成パイプラインです。

1. Obsidian の Markdown を読む
2. Front Matter を解釈・検証する
3. 内部リンクや埋め込みを解決する
4. Markdown を公開用 HTML に変換する
5. 記事一覧やカテゴリ一覧などの index データを生成する
6. 成果物を S3 にアップロードする
7. Leptos SSR サーバーがそれを読んで公開する

この境界に合わせて、Obsidian 読み取り、Front Matter 解釈、Markdown 変換、成果物生成、S3 配置といった publisher 側の実装は `apps/` に集約し、`crates/` は公開成果物を読む blog 側と共有契約に寄せます。

### ビルド時変換

Markdown から HTML への変換はランタイムではなくビルド時に行います。SSR サーバーは、変換済みの HTML と index データを読み、ルーティング、レイアウト、meta 情報の組み立てに集中します。

### Rust らしい責務分割

- 純粋ロジックと I/O を分離する
- 外部境界は trait で薄く切る
- 型で不正状態を減らす
- 単一バイナリでの運用性を優先する
- ビルド時に解決できる責務をランタイムへ持ち込まない

## 最終的な workspace 像

```text
okawak_blog/
├── crates/
│   ├── domain/            # 公開成果物契約と純粋ルール
│   ├── infrastructure/    # Leptos サーバー側の S3 / cache / runtime adapter
│   ├── server/            # 公開成果物を読む統合バックエンド
│   └── web/               # Leptos SSR 公開 UI
├── apps/
│   ├── obsidian_uploader/ # publisher CLI の中心
│   └── ...                # parser / renderer / uploader など publisher 側 crate 群
├── docs/
│   ├── architecture/
│   └── adr/
├── service/
└── terraform/
```

### 各層の責務

- `crates/domain`: 公開成果物契約と純粋関数を中心にした共有ドメイン層
- `crates/infrastructure`: Leptos サーバーが公開成果物を読むための S3 / cache / runtime adapter
- `crates/server`: S3 上の成果物を読んで配信する統合バックエンド
- `crates/web`: Leptos SSR の公開 UI
- `apps/obsidian_uploader` と `apps/*`: Obsidian 読み取り、Markdown 変換、成果物生成、アップロードを担う publisher 側

## 公開成果物のイメージ

```text
site/
├── articles/
│   ├── <slug>.html
│   └── index.json
├── categories/
│   ├── tech.json
│   ├── daily.json
│   └── ...
├── tags/
│   └── index.json
├── assets/
│   └── ...
└── metadata/
    └── site.json
```

publisher はこれらの成果物を生成し、SSR サーバーはそれらを読んでページを返します。

## データフロー

```text
Obsidian repo
  -> publisher
  -> HTML / index JSON / assets を生成
  -> AWS S3
  -> Leptos SSR server
  -> Browser
```

## 運用モデル

- VPS 上で Rust 製サーバーバイナリを `systemd` service として起動する
- `nginx` を前段に置いて HTTPS 終端とリバースプロキシを担当させる
- アプリケーション本体は単一バイナリとして扱う
- SSR サーバーは S3 上の成果物を読み、必要に応じて静的ファイルも配信する

## 開発原則

- `domain` 層は純粋関数のみとし、I/O と `async` を持ち込まない
- 大きめの実装に入る前に GitHub Issue に実装方針とタスク分解を書く
- 実装中の進捗や判断は GitHub Issue / PR に残し、恒久的な知識だけを `docs/architecture/` や ADR に昇格する
- 重要な設計判断は ADR として残す
- `terraform/` は読み取り専用とし、編集やコマンド実行を行わない

## 開発コマンド

現在のリポジトリで利用する主要コマンドは以下です。

```bash
cargo make dev
cargo make integrated-dev
cargo make watch
cargo make format
cargo make test
cargo make test-domain
cargo make test-server
cargo make test-web
cargo make clippy
cargo make check
cargo make check-deps
```
