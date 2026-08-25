# site/server

production `server` binaryを持つ単一のTopcoat application crateです。runtimeの起動、router composition、API、HTTP cache、assetと、公開UI、SSR route、metadataを所有します。Markdown変換は行わず、UI moduleはstorage非依存の`PageLoader`からdomainのpage documentを受け取ってhome、about、category、articleを表示します。`ArtifactPageLoader`だけが`site/infra`のartifact readerを利用します。

browser E2Eはserverとartifact readerを含む公開サイト全体を対象とするため、repository rootの[`e2e/`](../../../e2e/README.md)に置いています。

## applicationとruntimeの境界

- `src/main.rs`: process設定、tracing subscriber、reader、listenerの初期化
- `src/app.rs`: `module_router!()`を呼ぶroute tree root。home route、global layer、app context、assetのcomposition
- `src/app/api.rs`と`src/app/api/*.rs`: URL構造に対応するhealth、readiness、互換記事一覧API
- `src/http_cache.rs`: release-aware validatorとconditional GET
- `src/page_loader.rs`: storage非依存のpage load port
- `src/artifact_page_loader.rs`: artifact readerをpage load portへ接続するadapter

## UIとassetの境界

- `src/app.rs`: `/`のhome pageとpage loader contextの共有
- `src/app/about.rs`: `/about`の固定page
- `src/app/category_name.rs`: `/{category_name}`のcategory page
- `src/app/category_name/article_slug.rs`: `/{category_name}/{article_slug}`のarticle page
- `src/article_card.rs`: 一覧routeが共有する記事card
- `src/shell.rs`: HTML shell、metadata、error view、生成contentのprogressive enhancement
- `src/assets.rs`: application所有のbundle asset登録
- `src/navigation.js`: client-side navigationとmobile menu
- `style/tailwind.css`: theme token、site chrome、Tailwind CSS入力
- `style/content.css`: `.content-prose`配下の生成HTML用plain CSS
- `build.rs`: `style/tailwind.css`をTopcoatのstylesheet assetへ変換するbuild integration

routeはTopcoatのmodule-derived pathを使い、Rustのmodule treeを公開URL構造へ対応させます。dynamic segmentは`path_param!()`で宣言し、route moduleに`mod.rs`は使いません。

productionはpackage直下の`build.rs`から`style/tailwind.css`をTopcoatのstandalone Tailwind integrationで生成します。Tailwind CSS、Topcoat runtime、site navigation JavaScript、faviconはTopcoat asset bundleからcontent-hash付きlocal URLで配信します。

KaTeX、highlight.js、Font Awesome、Noto Sans JPはversion固定またはURL固定のCDN資産として維持します。KaTeXにはSRIを付与します。これらは数式・syntax highlight・icon・fontの段階的な装飾であり、SSR本文とnavigationの基本機能は外部CDNの成功に依存しません。

Sass、Stylance、CSS module、Node / BunによるCSS生成工程はありません。formatにはrepository rootの`mise run format`を使い、`cargo fmt`に加えて`topcoat fmt`で`view!` macroを整形します。buildには`mise run build-project`、確認には`mise run test-server`と`mise run test-e2e`を使います。
