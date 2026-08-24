# site/web

Topcoatによる公開UI、SSR route、metadataを提供するcrateです。Markdown変換は行わず、SSR時に`ArtifactReader`から公開artifactを読み取ってhome、about、category、articleを表示します。storageの実装詳細は`site/infra`、runtimeの起動とreader注入、API、HTTP cacheは`site/server`が担当します。

browser E2Eはweb crate単体ではなく、serverとartifact readerを含む公開サイト全体を対象とするため、repository rootの[`e2e/`](../../../e2e/README.md)に置いています。

## UIとassetの境界

- `src/topcoat_pages.rs`: Topcoat route / component、site shell、metadata
- `src/topcoat_navigation.js`: client-side navigationとmobile menu
- `src/generated_content.rs`: artifact本文へ適用するKaTeX / highlight.jsの初期化
- `style/tailwind.css`: theme token、site chrome、Tailwind CSS入力
- `style/content.css`: `.content-prose`配下の生成HTML用plain CSS

productionは`style/tailwind.css`をTopcoatのstandalone Tailwind integrationで生成します。Tailwind CSS、Topcoat runtime、site navigation JavaScript、faviconはTopcoat asset bundleからcontent-hash付きlocal URLで配信します。

KaTeX、highlight.js、Font Awesome、Noto Sans JPはversion固定またはURL固定のCDN資産として維持します。KaTeXにはSRIを付与します。これらは数式・syntax highlight・icon・fontの段階的な装飾であり、SSR本文とnavigationの基本機能は外部CDNの成功に依存しません。

Sass、Stylance、CSS module、Node / BunによるCSS生成工程はありません。buildにはrepository rootの`mise run build-project`、確認には`mise run test-web`と`mise run test-e2e`を使います。
