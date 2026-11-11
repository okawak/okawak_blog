# server

production `blog-server` binaryを持つ単一のTopcoat application crateです。runtimeの起動、router composition、API、HTTP cache、assetと、公開UI、SSR route、metadataを所有します。Markdown変換は行わず、UI moduleはstorage非依存の`PageLoader`からblog-domainのpage documentを受け取ってhome、about、category、articleを表示します。`ArtifactPageLoader`だけが`crates/blog-infra`のartifact readerを利用します。

browser E2Eはserverとartifact readerを含む公開サイト全体を対象とするため、repository rootの[`e2e/`](../../e2e/README.md)に置いています。

## applicationとruntimeの境界

- `src/main.rs`: process設定、公開BaseUrlの検証、tracing subscriber、reader、listenerの初期化
- `src/app.rs`: `module_router!()`を呼ぶroute tree root。home route、global layer、共有context、assetのcomposition
- `src/app/api.rs`と`src/app/api/*.rs`: URL構造に対応するhealth、readiness、互換記事一覧APIとOpenAPI文書
- `src/http_cache.rs`: release-aware validatorとconditional GET
- `src/page_loader.rs`: storage非依存のpage load port
- `src/artifact_page_loader.rs`: artifact readerをpage load portへ接続するadapter

## API仕様

`GET /api/openapi.json`はTopcoatのOpenAPI 3.1生成機能で、登録済みrouteの`Operation`からAPI仕様をJSONとして返します。公開URLには起動時に検証した`BaseUrl`を使います。文書生成はartifactを読み取らないため、storage障害中も利用でき、content releaseのvalidatorは付けません。

対象は既存の`GET /api/articles`、`GET /api/health`、`GET /api/ready`です。記事一覧は日本語の互換APIで、S3 snapshot cacheが有効な場合のconditional GET（304）と読取失敗（500）を記述します。healthはprocessの生存だけ、readyはsite metadataを読めること（200 / 503）を表します。本文schemaはblog-domainの`schema` featureを通じて実際の公開artifact型から導出し、同じ構造を別DTOへ複製しません。

SSR page・asset・内部runtime route・OpenAPI文書自身には`Operation`を付けず、API仕様の対象にしません。OpenAPIは説明であり、実際のvalidation・認証・cache制御を変更するものではありません。

## UIとassetの境界

- `src/app.rs`: `/`のhome pageとpage loader contextの共有
- `src/app/about.rs`: `/about`の固定page
- `src/app/category_name.rs`: `/{category_name}`のcategory page
- `src/app/category_name/article_slug.rs`: `/{category_name}/{article_slug}`のarticle page
- `src/article_card.rs`: 一覧routeが共有する記事card
- `src/category_articles.rs`: signalをサーバー側で読み、カテゴリ記事一覧だけを再描画するshard
- `src/article_filter.rs`: タイトル・説明・タグによる記事絞り込みの純粋ロジック
- `src/components/`: `topcoat ui`が管理し、site向けに調整したvendored UI component
- `src/shell.rs`: HTML shell、metadata、error view、生成contentのprogressive enhancement
- `src/navigation.js`: shellへ埋め込み、リンク選択・履歴移動・画面幅切替でmobile popoverを閉じる補助処理
- `src/assets.rs`: application所有のbundle asset登録
- `src/icons.rs`: 同梱したGitHub Octiconsの単一SVG（[MITライセンス](licenses/Octicons-MIT.txt)）をTopcoatのicon componentへ渡す
- `components.toml`: Topcoat UIのtheme・component install state
- `styles.css`: Topcoat UI theme token、site chrome、`.content-prose`配下の生成HTML用styleをまとめたTailwind CSS入力
- `build.rs`: `styles.css`をTopcoatのstylesheet assetへ変換するbuild integration

routeはTopcoatのmodule-derived pathを使い、Rustのmodule treeを公開URL構造へ対応させます。dynamic segmentは`module_param!()`で宣言し、`category_name(cx)`など宣言名のaccessorで値を取得します。カテゴリは`module_param!(category_name: Category else not_found)`で解析します。不正なカテゴリ・slugとpageの欠落はTopcoatの`NotFoundError`を返し、共通layerで日英の404を描画します。描画もconditional GET layerが設定したsnapshotの内側で行います。content assetも`RouterErrorExt`で404へ変換しますが、siteのHTML shellは描画しません。記事slugの`.html`正規化とstorage errorの500表示はserverが管理します。route moduleに`mod.rs`は使いません。

Topcoat frameworkとCLIはworkspaceで同じversionに揃え、shardは明示したendpointを`.route(category_articles)`で登録します。applicationのglobal layerを先に追加し、最後に`.layer(RuntimeLayer::new())`を登録して、application layerより前に再描画POSTをGETへrewriteします。`TrailingSlash::Redirect`により、末尾スラッシュ付きの公開URLはqueryを維持した308で宣言済みのURLへリダイレクトします。

`DynPageLoader`と`DynArtifactReader`はラッパーを挟まずTopcoatの統合contextへ直接登録し、`context::<T>(cx)` / `try_context::<T>(cx)`で取得します。共有値はrouterの`.with(...)`、release snapshotと対応するPageLoaderは`cx.with_many((snapshot, loader))`で追加します。requestの値が共有値をshadowするため、UIとAPIは同じsnapshotを参照します。先読みの既定値も`.with(PrefetchMode::Intent)`で設定します。

公開URLの基準は起動時に`OKAWAK_BLOG_SITE_ORIGIN`から一度だけ読み、Topcoatの`BaseUrl`としてrouterへ渡します。未設定・空文字は`https://www.okawak.net`を使い、HTTP(S)の絶対URLでない値やquery・fragmentを含む値はartifact初期化前にエラーにします。canonical・Open Graph URL・hreflangは同じ`BaseUrl`を使います。URL結合は`BaseUrl::join()`へ任せ、日本語homeだけは既存契約に合わせて末尾slashなしの`BaseUrl::as_str()`を使います。

runtimeのpage再描画はpage自身のURLに`X-Topcoat-Runtime: true`を付けてPOSTし、GETへ内部rewriteします。conditional GETでは`request::original_method(cx)`を参照し、再描画応答に通常pageのvalidatorを適用しません。

カテゴリの記事絞り込みはshard内の`signal(cx, String::new)`とtracked readで構成します。タイトル・説明・タグを大文字小文字を区別せず部分一致で検索し、入力はサーバー側でも先頭100文字に制限します。`/_topcoat/shards/category-articles`へ渡すカテゴリ・言語引数も再描画時に検証します。固定の引数は通常の値で渡します。ページと初期shardは`#[memoize]`で同じpage documentを共有し、再描画では`PageLoader`から再取得します。記事・section・タグのloopには`#[key(...)]`で安定したidentityを与えます。DOMにも記事・sectionの安定したIDを付け、DOM morphにより、入力フォーカスとshard外の状態を維持します。JavaScript無効時も初期HTMLの全記事とリンクを利用できます。

各pageは読み込み済みpresentationからTopcoatの`Metadata`を直接組み立て、shellへ渡します。shellは言語・公開path・公開locale情報を別の引数で受け取り、canonicalとサイト名を補って`cx.with(...)`へ登録し、`topcoat::metadata::tags()`をhead先頭に描画します。title、description、canonical、Open Graph、Xのsummary cardを一括出力し、`og:site_name`も日英のUI辞書に合わせます。Open Graphのtitle・description・URLはpage metadataのfallbackを使い、article / websiteは`OpenGraphType`で区別します。`og:locale`と、公開済み翻訳に限ったhreflangはshellで補います。metadataのための追加読込は行わず、本文と同じpresentationを使います。

productionはpackage直下の`build.rs`から`styles.css`をTopcoatのstandalone Tailwind integrationで生成します。Tailwind CSS、Topcoat runtime、faviconはTopcoat asset bundleからcontent-hash付きlocal URLで配信します。

記事card・カテゴリ・header・言語切替のlinkは`runtime::link_attrs`により、SSR結果でdocument・title・metadataを更新するruntime navigationを使います。既定の`PrefetchMode::Intent`でhover・focus・touch時に先読みし、cookieを保存する言語切替と日本語homeの`?lang=ja` linkは`Never`にします。言語選択はruntime POSTがrewriteされたGETにも適用し、validatorの判定だけはoriginal methodを使います。言語切替の表示はTopcoat UIの`toggle_group`を使います。生成Markdown内のanchor（text fragmentを含む）と外部linkはブラウザ標準のnavigationを維持します。JavaScript無効時はすべて通常のanchorとして利用できます。

検索fieldは`field` / `field_label` / `field_description` / `input`を使い、`element_id(cx)`で再描画後も安定したIDを生成し、labelと入力、検索対象の説明を関連付けます。category・tag・countは`badge`、記事一覧は`card`、状態表示は`alert`で構成します。headerの操作とnavigation linkは`button`のstyleを共有し、GitHub linkは`tooltip`で補足します。tooltipも生成IDで`aria-describedby`を関連付け、`floating`のCSS anchor positioningと配置fallbackで画面端を避けます。mobile menuは公式`popover` / `popover_trigger` / `popover_content`を使い、`element_id`でtriggerとpanelを関連付けます。開閉、外側クリック、Escape、focus復帰と支援技術へのexpanded状態はブラウザーのPopover APIが担当し、JavaScriptなしでも操作できます。日英の開閉ラベルとアイコンは`:popover-open`に連動します。desktopとmobileは同じnavigation componentからリンクを生成します。`src/navigation.js`をshellへ埋め込み、リンク選択・履歴移動・desktop幅への切替でpanelを閉じます。panelのIDはrequest pathで分けます。履歴移動とscrollの復元はTopcoat runtimeが担当します。

件数と記事の日付はTopcoatの`l10n-number` / `l10n-datetime`で明示的な日英localeへ整形します。件数は桁区切りを付け、日付はartifactのoffsetにおける日を維持して時刻を表示しません。server内でblog-domainの`Locale`をTopcoatのlocaleへ変換し、共有modelにはframework依存を持ち込みません。UI辞書は引き続き`locales/ui.json`を使い、翻訳追跡と日本語fallbackを維持します。

GitHubアイコンはTopcoatのicon componentでinline SVGを描画し、icon fontや外部icon setの取得は行いません。リンクにaccessible nameを付け、装飾のSVGは支援技術から隠します。端末間の字体を揃えるNoto Sans JPはGoogle Fontsの可変ウェイト範囲`400..700`でCSSの重複を抑え、HTML headから直接参照します。`display=swap`でフォント取得中も本文を表示します。

KaTeX、highlight.jsはversion固定のCDN資産として維持し、KaTeXにはSRIを付与します。数式・syntax highlight・fontは段階的な装飾であり、SSR本文とnavigationの基本機能は外部CDNの成功に依存しません。runtime navigationはscriptを再実行しないため、document rootのMutationObserverでDOM更新を検出して数式・codeを再装飾します。装飾中はobserverを停止し、自身のDOM変更で更新が循環しないようにします。

Sass、Stylance、CSS module、Node / BunによるCSS生成工程はありません。formatにはrepository rootの`mise run format`を使い、`topcoat fmt --rustfmt crates`でRustと`view!` macroをまとめて整形します。`mise run format-check`は`--check`で変更せずに検証し、CIでも実行します。rootの`rustfmt.toml`はeditionを指定し、`rust-analyzer.toml`は保存時の整形も同じformatterへ揃えます。buildには`mise run build-project`、確認には`mise run test-blog-server`と`mise run test-e2e`を使います。

## 開発時のホットリロード

`mise run dev-local`、`mise run dev`、`mise run dev-fixture`は`topcoat dev`でRust・styleの変更を監視し、再buildとasset bundleの更新を行います。`dev-local`のpublishは起動前の1回だけです。公開Markdownを変更した場合はtaskを再起動してartifactを生成し直します。

shellの`topcoat::dev::script()`は`topcoat dev`実行時だけ出力されます。再build後も、対応するform入力とidentityが一致するsignal（検索語）を保ちます。componentやsignalの呼出位置が変わると状態がリセットされることがあり、script・base URL・doctypeの変更はfull reloadになります。productionと通常E2Eは従来どおりbundle済みbinaryを直接起動します。
