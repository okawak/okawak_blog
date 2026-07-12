# site/web

Leptosによる公開UIとSSR routeを提供するcrateです。Markdown変換は行わず、SSR時に`ArtifactReader`から公開artifactを読み取ってhome、about、category、articleを表示します。

browser E2E は web crate 単体ではなく、server と artifact reader を含む公開サイト全体を対象とするため、リポジトリルートの [`e2e/`](../../../e2e/README.md) に置いています。
