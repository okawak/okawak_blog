# site/web

Leptosによる公開UIとSSR routeを提供するcrateです。Markdown変換は行わず、SSR時に`ArtifactReader`から公開artifactを読み取ってhome、about、category、articleを表示します。

browser E2E は web crate 単体ではなく、server と artifact reader を含む公開サイト全体を対象とするため、リポジトリルートの [`e2e/`](../../../e2e/README.md) に置いています。

## スタイリング

- Rust/UI由来のprimitiveは`src/components/ui/`に置く
- site固有のcomponentとroute layoutはTailwind classで構成する
- theme tokenとbase styleは`style/tailwind.css`をsource of truthにする
- artifactの`inner_html`は`.content-prose`で囲み、`style/content.css`のplain CSSだけを適用する

productionは`style/tailwind.css`をTopcoatのstandalone Tailwind build integrationで生成し、Topcoat asset bundleから配信します。build、通常E2E、S3 smokeは`cargo-leptos`やNode / BunのCSS build toolへ依存せず、Leptos JavaScript / WebAssemblyを生成しません。Sass、Stylance、CSS moduleの生成工程はありません。完全撤去まで残るlegacy Leptos依存のinstall・更新確認にはrepository rootの`mise run web-*`を使います。
