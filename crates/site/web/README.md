# site/web

Leptosによる公開UIとSSR routeを提供するcrateです。Markdown変換は行わず、SSR時に`ArtifactReader`から公開artifactを読み取ってhome、about、category、articleを表示します。

browser E2E は web crate 単体ではなく、server と artifact reader を含む公開サイト全体を対象とするため、リポジトリルートの [`e2e/`](../../../e2e/README.md) に置いています。

## スタイリング

- Rust/UI由来のprimitiveは`src/components/ui/`に置く
- site固有のcomponentとroute layoutはTailwind classで構成する
- theme tokenとbase styleは`style/tailwind.css`をsource of truthにする
- artifactの`inner_html`は`.content-prose`で囲み、`style/content.css`のplain CSSだけを適用する

現行productionのLeptos buildは`style/tailwind.css`を`cargo-leptos`でcompileします。移行用Topcoat runtimeは同じ入力をTopcoatのstandalone Tailwind build integrationで生成し、Topcoat asset bundleから配信します。通常Topcoat E2EはNode / BunのCSS build toolへ依存しません。Sass、Stylance、CSS moduleの生成工程はありません。production用web依存のinstall・更新確認はrepository rootの`mise run web-*`を使い、`mise run build-project`は`web-install`を先に実行します。
