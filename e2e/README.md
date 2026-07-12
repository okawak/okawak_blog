# Browser E2E

公開サイト全体を対象とする Playwright E2E です。`crates/site/web` 単体ではなく、`crates/site/server` と `crates/site/infra` の artifact reader まで通すため、リポジトリルートに置いています。

依存管理には、web crate と同じく Bun を使います。通常はリポジトリルートから `mise` task を実行してください。

```bash
# 初回準備（依存と Chromium をインストール）
mise run e2e-install-browser

# E2E を実行
mise run test-e2e
```

依存の更新と確認には `mise run e2e-update` / `mise run e2e-outdated` を使います。

テストは `fixtures/site` の固定 artifact だけを読みます。private Obsidian submodule、S3、AWS credentials には依存しません。Playwright が `127.0.0.1:8008` で専用の Leptos サーバーを起動し、home、about、category、article、404 status、metadata、hydration 後の route 遷移を Chromium で検証します。

失敗時の trace は `e2e/test-results` に保存されます。
