# site/web

Topcoatによる公開UI、SSR route、metadataを提供するcrateです。Markdown変換やartifact読取は行わず、storage非依存の`PageLoader`からdomainのpage documentを受け取ってhome、about、category、articleを表示します。`PageLoader`のartifact-backed実装、runtimeの起動、reader注入、API、HTTP cacheは`site/server`が担当します。

browser E2Eはweb crate単体ではなく、serverとartifact readerを含む公開サイト全体を対象とするため、repository rootの[`e2e/`](../../../e2e/README.md)に置いています。

## UIとassetの境界

- `src/page_loader.rs`: storage非依存のpage load port
- `src/pages.rs`: 公開routeのre-exportとpage loader contextの共有
- `src/pages/{home,article,category,page}.rs`: page種別ごとのrouteと固有component
- `src/article_card.rs`: 一覧routeが共有する記事card
- `src/shell.rs`: HTML shell、metadata、error view
- `src/assets.rs`: application所有のbundle asset登録
- `src/navigation.js`: client-side navigationとmobile menu
- `src/content_enhancement.rs`: artifact本文へ適用するKaTeX / highlight.jsの初期化
- `style/tailwind.css`: theme token、site chrome、Tailwind CSS入力
- `style/content.css`: `.content-prose`配下の生成HTML用plain CSS
- `build.rs`: `style/tailwind.css`をTopcoatのstylesheet assetへ変換するbuild integration

productionは`style/tailwind.css`をTopcoatのstandalone Tailwind integrationで生成します。Tailwind CSS、Topcoat runtime、site navigation JavaScript、faviconはTopcoat asset bundleからcontent-hash付きlocal URLで配信します。

KaTeX、highlight.js、Font Awesome、Noto Sans JPはversion固定またはURL固定のCDN資産として維持します。KaTeXにはSRIを付与します。これらは数式・syntax highlight・icon・fontの段階的な装飾であり、SSR本文とnavigationの基本機能は外部CDNの成功に依存しません。

Sass、Stylance、CSS module、Node / BunによるCSS生成工程はありません。formatにはrepository rootの`mise run format`を使い、`cargo fmt`に加えて`topcoat fmt`で`view!` macroを整形します。buildには`mise run build-project`、確認には`mise run test-web`と`mise run test-e2e`を使います。
