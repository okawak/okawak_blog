# site/web

Leptosによる公開UIとSSR routeを提供するcrateです。Markdown変換は行わず、SSR時に`ArtifactReader`から公開artifactを読み取ってhome、about、category、articleを表示します。

## Browser E2E

E2Eは`end2end/fixtures/site`の固定artifactだけを使います。private Obsidian submodule、S3、AWS credentialsには依存しません。

初回準備:

```bash
cd crates/site/web/end2end
npm ci
npx playwright install chromium
```

実行:

```bash
npm test
```

Playwrightが`127.0.0.1:8008`でE2E専用のLeptosサーバーを起動します。失敗時のtraceは`end2end/test-results`に保存されます。

現在のfirst cutはChromiumのみを対象とします。home、about、category、article、404 status、metadata、hydration後のroute遷移を検証します。
