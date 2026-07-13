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

## ローカルから実S3を確認する

CIへS3接続を組み込む前の手動統合確認には、通常の固定artifact E2Eとは分離した`test-e2e-s3`を使います。このtaskはAWS SDKの標準credential chainでS3を読み、`/api/ready`、home、article index、実articleのSSR表示とmetadataを確認します。

bucket名やcredentialはrepositoryへ保存せず、実行時に渡してください。task自身はAWS CLIを実行せず、server内のAWS SDKがcredentialを読みます。`aws configure --profile blog-s3`などでshared config / credentials fileへ設定済みなら`AWS_PROFILE=blog-s3`で選択でき、省略時はdefault profileが使われます。regionは`AWS_REGION`、`AWS_DEFAULT_REGION`、またはprofileで設定できます。

```bash
# 初回だけ。すでにprofileを設定済みなら不要
aws configure --profile blog-s3
```

profileを使わず、`AWS_ACCESS_KEY_ID`、`AWS_SECRET_ACCESS_KEY`、必要に応じて`AWS_SESSION_TOKEN`を実行環境へ設定する方法でも動作します。現在の`aws-config` dependencyはdefault featureを無効化しているため、SSOや`credential_process`を使うprofileはこのローカルtestの対象外です。

```bash
# 開発サーバーを起動してブラウザから手動確認
AWS_PROFILE=<read-only-profile> \
AWS_REGION=ap-northeast-1 \
OKAWAK_BLOG_ARTIFACT_BUCKET=<bucket> \
mise run dev

# Playwright smoke testを実行
AWS_PROFILE=<read-only-profile> \
AWS_REGION=ap-northeast-1 \
OKAWAK_BLOG_ARTIFACT_BUCKET=<bucket> \
mise run test-e2e-s3
```

artifactがbucket root以外にある場合だけ、先頭と末尾の`/`を除いたprefixも渡します。readerは`<prefix>/current.json`を起点にrelease artifactを解決します。

```bash
OKAWAK_BLOG_ARTIFACT_PREFIX=<prefix> \
AWS_PROFILE=<read-only-profile> \
AWS_REGION=ap-northeast-1 \
OKAWAK_BLOG_ARTIFACT_BUCKET=<bucket> \
mise run test-e2e-s3
```

credentialには対象keyへの`s3:GetObject`だけを付与したread-only profileを推奨します。`/api/health`が成功して`/api/ready`が失敗する場合は、profile/region、bucket/prefix、`current.json`とrelease artifactへのGetObject権限を確認してください。

このtestは実データとAWS credentialに依存するため、通常の`mise run test-e2e`やCIからは実行されません。
