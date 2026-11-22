# okawak_blog アーキテクチャ

## 文書の役割

この文書は、現行システムの責務、依存方向、境界をまたぐ不変条件を定める。入力schemaの詳細は[Obsidian template](../content/obsidian-template.md)と[公開Markdown契約](../content/public-markdown.md)、crate内部の利用方法は各crateのREADME、本番操作は[operations](../operations/README.md)を正本とする。実装計画と完了済みの移行経緯はIssue / PRに残す。

okawak_blogは、Obsidianで書いたMarkdownを公開artifactへ変換し、Topcoat SSRで配信する静的コンテンツ公開基盤である。Markdown変換と公開可否の判定はビルド時に完了させ、runtimeはartifact読取、routing、metadata、UIへ集中する。

## システム概要

1. ローカルのexportがprivate Obsidianから公開対象を抽出・正規化する
2. 公開文章、タグ表示名、UI文言の必要な差分だけをローカルCodexで翻訳する
3. review済みの多言語Markdownと辞書をGitへ確定する
4. publishが言語別HTML・JSONと公開path対応表を生成する
5. GitHub Actionsがimmutable S3 releaseへuploadし、artifactと表示を検証する
6. 検証成功後だけcurrent.jsonを新releaseへ切り替える
7. VPS上のTopcoat serverがS3 artifactを読み、Cloudflare Tunnel経由で配信する

~~~mermaid
flowchart LR
    A[Private Obsidian] --> B[export]
    B --> C[Public Markdown and catalogs in Git]
    C --> D[publish]
    D --> E[Immutable S3 release]
    E --> F[infra reader]
    F --> G[Topcoat server]
    G --> H[Cloudflare Tunnel]
    H --> I[Browser]
~~~

private ObsidianとAI認証はローカルのexportだけが使う。GitHub Actions、publish、runtimeはGit管理済みの公開入力だけを扱う。

## Workspaceと依存方向

内部パッケージとディレクトリは`blog-`接頭辞で統一し、Rustからは`blog_domain`、`blog_export`、`blog_publish`、`blog_infra`、`blog_server`として参照する。全パッケージに`publish = false`を設定し、Cargo registryへ公開しない。実行ファイルは`blog-export`、`blog-publish`、`blog-server`である。

~~~text
okawak_blog/
├── obsidian/          # private入力のgit submodule
├── content/           # export済みの公開Markdown・タグ辞書
├── translation.json   # 記事・タグ・UI共通の翻訳設定
├── crates/
│   ├── blog-export/
│   ├── blog-domain/
│   ├── blog-publish/
│   ├── blog-infra/
│   └── blog-server/
├── e2e/
├── docs/
├── scripts/
├── service/
└── terraform/
~~~

obsidian/は外部入力であり、crateの内部資産にしない。公開入力のcontent/と翻訳設定もworkspace rootで管理し、変換処理とデータを分離する。

~~~mermaid
flowchart TB
    Export[crates/blog-export] --> BlogDomain[crates/blog-domain]
    Publish[crates/blog-publish] --> BlogDomain
    Infra[crates/blog-infra] --> BlogDomain
    Server[crates/blog-server] --> BlogDomain
    Server --> Infra
~~~

exportとpublishはruntime依存ではない。serverはinfraをapplication compositionで利用するが、UI componentはstorageへ直接依存しない。

### crates/blog-export

- private Obsidianの公開対象抽出、ノート参照・embedの正規化、安定IDの管理
- 記事・タグ・UIに共通する差分翻訳、手動編集の保護、更新候補の生成と採用
- public fragmentだけを渡すCodex実行境界と、記事・タグを一括更新するfilesystem transaction。UI辞書は別transactionで更新する
- private入力が読めない場合は停止し、公開Markdownだけの処理へ切り替えない

内部は機能単位のmoduleで構成する。`operation`がexport・辞書翻訳・候補採用の各操作の入口、処理順、lock・transaction境界を所有し、`vault`が公開対象の抽出・正規化、`translation`が記事・辞書の更新計画、翻訳の生成、候補の検証・反映、`output`が公開ファイルの同期・退避を担う。CLIは引数の解釈・操作の選択・結果表示を担当し、`lib.rs`から公開された`operation`の関数を呼ぶ。`vault`は正規化済みの文書を返し、private入力の情報を翻訳処理へ渡さない。

記事画像の圧縮・アップロードはObsidianのS3 Image Uploaderが担当し、配信用artifactとは別のS3 bucketに保存してCloudFrontから配信する。Markdownの画像記法`![説明](...)`はHTTP(S)画像専用とする。exportは本文の画像URLを保持し、ローカル画像の収集・コピー・同期を行わない。画像ファイルはGit管理せず、ローカル画像参照はexport時に拒否する。ノート参照には通常のMarkdownリンクかWikiLink・Wiki embedを使う。参照形式のリンク・画像は使わず、未使用も含め参照定義を検出した時点でexportをエラーにする。

`content`はI/Oを持たない文書codec・Markdown設定・fingerprint、`filesystem`はschemaに依存しない走査・lock・stagingを所有する。翻訳の更新判定は純粋な`translation::plan`、応答cacheは`translation::cache`、Codex実行は`translation::codex`に分ける。記事と辞書は同じ更新計画を使い、候補衝突の検証をAI呼び出しより前に完了する。下位のcontent / filesystem / outputからvault / translationへ依存させない。

操作と翻訳状態の詳細は[export README](../../crates/blog-export/README.md)を参照する。

### crates/blog-domain

- 公開コンテンツ、artifact、release pointer、page documentの純粋な共有契約
- category、slug、locale、pathなどの値と検証規則
- artifactから表示用documentを組み立てる純粋ロジック
- I/O、async、AWS SDK、HTTP frameworkを持たない

任意の`schema` featureは、公開記事一覧型のJSON schemaを`schemars`で導出する。HTTP frameworkやOpenAPIのroute設定はblog-serverに置く。

公開Markdownのタイトル・日時・タグID・digest・section pathはblog-domainの値型が生成時・deserialization時に検証する。`PublicContentMeta::validate()`はschema versionとフィールド間の整合性を検証する。exportのvault入力も同じ値型を使い、publishへ検証済みの値を渡す。記事のメタデータはArticleMetaに統一し、一覧用に同じフィールドのmodelを重複定義しない。CategoryIndexはPublishableCategoryLandingを本文ごと保持し、artifact生成時にlandingを検索し直さない。

### crates/blog-publish

- 公開Markdownの読込、種別分類、安定IDによるlink解決、HTML変換、安全化、artifact生成
- bookmark enrichmentを外部境界として注入し、pipelineが処理順を所有する
- 全言語を一時領域で生成・検証した後だけ既存siteと入れ替える
- lib.rsはmodule宣言とre-exportに限定し、crate外APIはpublish entrypoint、bookmark enricher、PublishError / Resultに限定する
- path処理の対応環境はmacOSとLinuxとする

入力、出力、検証方法は[publish README](../../crates/blog-publish/README.md)を参照する。

### crates/blog-infra

- ArtifactReader / ArtifactSnapshotによるstorage非依存の読取境界
- local filesystemとS3のreader、release解決、cache、source設定
- vault読取、Markdown変換、S3 upload、HTTP response、UIを扱わない

### crates/blog-server

- 単一のproduction server binary、Topcoat application、公開route、metadata、UI、asset、API
- src/app.rsをmodule_router!()のrootとし、app/のfile moduleをURL構造へ対応させる
- storage非依存のPageLoaderと、artifact readerを接続するArtifactPageLoader
- release-aware conditional GET、health / readiness、readerの生成と注入
- UI文言・カテゴリ表示名の辞書と、言語選択・切替

file単位の構成、Topcoat、styleの境界は[server README](../../crates/blog-server/README.md)を参照する。

### E2Eと運用

e2e/はserverとartifact readerを通るbrowser E2Eを所有する。通常CIは固定fixtureを使い、private submodule、AI、AWSへ依存しない。実S3 smokeはローカル手動確認と公開workflowのgateに限定する。

scripts/tests/は運用スクリプト、service/tests/はsystemd unitを検証し、mise run test-serviceでまとめて実行する。terraform/は通常のrepository作業では読み取り専用とする。

## コンテンツ境界

private Obsidianは執筆形式、content/はexportとpublish間の公開契約である。Obsidian固有frontmatter、WikiLink、embed、private pathをpublishへ渡さない。公開Markdownはschema version、安定ID、locale、kind、日時、元pathのdigestを持ち、翻訳版が翻訳追跡情報を持つ。正確なfieldと組合せは[公開Markdown契約](../content/public-markdown.md)を正本とする。

content kindはarticle、category、page、homeの4種類である。articleはcategoryと同名の入力directoryに置き、categoryからの相対pathをsection_pathとして一覧のgroupingに使う。URLはsection_pathに依存しない。執筆例とdirectory規則は[Obsidian template](../content/obsidian-template.md)に置く。

記事と内部linkは翻訳後も変わらないcontent IDで対応づける。タイトル、元file path、翻訳結果から公開identityを再計算しない。publishは対象言語のcontentが配信可能なら同じ言語、なければ日本語のURLへ解決する。未解決参照、曖昧な参照、公開対象外への参照はexportまたはpublishで失敗させる。

タグIDは記事frontmatterに保持し、表示名はcontent/tags.jsonから同じcontent releaseへ含める。UI文言とカテゴリ表示名はserver辞書で管理し、server binaryと一緒に配備する。blog-domainは表示文言を持たず、識別子と共有schemaだけを所有する。

## Artifactとrelease

blog-domainがページ種別と安定IDから公開ルートを決め、全言語に同じ保存規則を適用する。公開URLから先頭のスラッシュを外し、Markdownは `.md`、ページJSONは `.json` を付ける。トップだけは言語ディレクトリの `index` を使う。

~~~text
site/
├── locales.json
├── content-assets/
└── <locale>/  # ja, en, zh-hans
    ├── index.json
    ├── about.json
    ├── <category>.json
    ├── <category>/<slug>.json
    └── _data/
        ├── articles.json
        ├── site.json
        └── tags.json
~~~

locales.jsonは同じreleaseで実在するpathとlocaleの対応、content-assets/は言語間で共有するcontent hash付きassetを持つ。各言語のmetadataはその言語で配信する記事だけを集計する。

記事のJSONは`ArticleArtifactDocument { summary, html }`であり、summaryは同じlocaleの_data/articles.jsonと一致する。publish前の検証で両者を照合する。local / S3 readerは指定category・slugと文書のidentityを検証する。記事JSONが存在しない場合は欠落、破損・権限エラーは失敗として返す。S3の欠落はSDKのNoSuchKeyまたはHTTP 404で判定し、エラーメッセージの文字列では判断しない。一覧や記事HTMLを代わりに読まない。保存形式はArticleArtifactDocumentに統一し、reader境界でidentity・メタデータ・本文を検証して既存のPublishableArticleへ変換する。cacheとSSRはこの検証済みmodelを使い、page builderで再検証しない。

index.jsonはhome fragmentがある場合だけ生成する。翻訳先のsubtreeは、その言語で配信可能なcontentがある場合だけ生成する。

日本語は記事1件以上、About、記事が属するcategory landingを必須とする。各翻訳先は翻訳履歴があり更新待ちでないcontentだけを採用し、landingがないcategoryの記事を掲載しない。翻訳版のAboutは任意である。全言語の生成とpage document構築に成功した場合だけsiteを入れ替え、古いfileを残さない。

本番は次のimmutable release構造を使う。

~~~text
current.json
releases/
└── <release-id>/
    ├── manifest.json
    └── site/
~~~

current.jsonとmanifest.jsonは同じversion付きpointer schemaを使い、release ID、artifact prefix、publisher / source / content commit、生成時刻を保持する。source / content commitは公開repositoryの同じcommitを示す。

公開workflowはmainから手動実行し、次の順序を守る。

1. 対象commitが最新mainであり、push起因CIが成功済みであることを確認する
2. Git管理済みの公開入力からsiteを生成・検証する
3. 新しいrelease prefixへuploadし、object集合と各言語の表示を検証する
4. 対象commitがまだ最新mainであることを再確認する
5. current.jsonを最後に切り替える

releaseは上書きせず、公開runをrepository単位で直列化する。失敗したrunは新releaseを残してもpointerを変更しない。この順序により、readerは不完全なuploadを選ばず、遅いrunが公開pointerを古いcommitへ戻すこともない。

## 公開URLと言語

公開ページの先頭は全言語で明示する。言語に依存しないページキーと安定IDを共有し、同じ記事は全言語で同じslugを使う。

| ページ | 日本語 | 英語 | 簡体字中国語 |
| --- | --- | --- | --- |
| Home | `/ja` | `/en` | `/zh-hans` |
| About | `/ja/about` | `/en/about` | `/zh-hans/about` |
| Category | `/ja/:category` | `/en/:category` | `/zh-hans/:category` |
| Article | `/ja/:category/:slug` | `/en/:category/:slug` | `/zh-hans/:category/:slug` |

`GET /` と `HEAD /` は公開済みのトップへ307で転送する。保存済みの選択、TopcoatのAccept-Language照合、英語、日本語の順で選ぶ。簡体字は言語タグ `zh-hans` に対応し、繁体字を指定するブラウザとは区別する。入口は `Vary: Accept-Language, Cookie` と `Cache-Control: no-store` を返す。各言語のURLはCookieやブラウザ設定に左右されない。旧日本語のAbout・カテゴリ・記事はクエリを保って対応する日本語URLへ308で転送する。API・assetは専用のパスを持つ。

言語切替は同じページの翻訳、対象言語トップの順で移動する。公開されていない言語は無効にする。明示的な選択はhost限定、Path=/、HttpOnly、SameSite=LaxのCookieに1年間保存する。各ページのcanonicalは明示URLとし、hreflangは同じリリース内で実在する翻訳間に付ける。未翻訳ページの直接URLはその言語の404、UI文言やタグ表示名の翻訳欠落・更新待ちは日本語表示とする。

## Pageとruntime境界

blog-domainのpage documentは言語やstorageに依存しない表示データを表す。serverのpage componentはPageLoaderから、page document、タグ表示名、言語別pathをまとめたpresentationを受け取る。ArtifactPageLoaderだけがinfraのsnapshotを読み、local / S3 readerをUIへ持ち込まない。serverはDynPageLoaderとDynArtifactReaderをTopcoatの統合contextへ直接登録し、requestのsnapshotに対応するloaderで共有loaderをshadowする。

1 request内のpage documentとmetadataは同じartifact snapshotから組み立てる。serverのpageは同じpresentationからTopcoatのMetadataを直接組み立て、shellがcanonical・サイト名を補って統合contextへ登録し、title・canonical・social metadataを描画する。公開URLは起動時に検証したTopcoatのBaseUrlに統一し、描画時に環境変数を読み直さない。hreflangとog:localeは公開locale情報から補う。home fragmentだけを任意とし、必要なartifactの欠落・読取失敗を空値へ変換しない。404とstorage errorはroute境界で異なるstatusとして扱う。/api/articlesは互換endpointとして維持し、page専用APIは増やさない。

公開routeはTopcoatのmodule treeから導出する。app.rsをrootとし、static segmentと`module_param!()`で宣言するdynamic segmentをfile moduleでURL構造へ対応させ、mod.rsは使わない。各言語は型付きのlanguage segmentと描画処理を共有する。release-aware HTTP cacheはroute固有処理ではなくglobal layerとして構成する。

responsive menuはTopcoat UIのPopoverとブラウザー標準の開閉・focus管理を使う。serverのshellがリンク選択・履歴移動・画面幅切替に伴う閉じる処理を補う。Topcoat runtimeはcategory内の記事絞り込み、UIが生成する内部linkのnavigationと先読みに使う。navigationはserverで描画したdocumentとmetadataを更新し、履歴・scrollを管理する。hover・focus・touchで先読みし、cookieを変更する言語選択linkは先読みしない。言語選択はruntime navigationのrewrite後GETにも適用する。生成Markdownのanchorは通常のbrowser navigationを維持する。初期HTMLには全記事と通常linkを含め、JavaScriptや外部CDNが失敗しても本文とnavigationを利用可能にする。UI component、asset、styleの詳細は[server README](../../crates/blog-server/README.md)を正本とする。

## Reader、cache、HTTP

ArtifactReaderは1処理で使うArtifactSnapshotを取得し、snapshotは同じreleaseのindex、metadata、HTML、辞書、assetを読む。

snapshotの文書・HTML・辞書は`Arc`、assetは`Bytes`で返し、cache hit時に本文や一覧全体を複製しない。S3のassetは取得した共有バイト列をHTTP応答まで保持する。記事単体はsnapshotの`read_article(category, slug)`でメタデータと本文を一括取得し、一覧の読込・検索を行わない。記事とassetは取得成功した値だけを保持し、欠落・失敗を保存しない。キー付きcacheの同時取得・エラー共有・キャンセル時の後片付けはMokaに委譲する。記事とassetのstorage結果（値・欠落・エラー）をMokaの成功・非保持へ対応付ける部分だけをinfraで補う。locale・カテゴリ・固定ページもMokaを使い、単一文書はTokio OnceCellを使う。cacheはlocale・releaseごとに分離し、release単位の寿命を維持するためMokaの容量・時間によるevictionは設定しない。

PageLoaderのpresentationは辞書とlocale catalogの共有参照を保持する。カテゴリのpageとshardはrequest内でpresentationを共有し、絞り込み時はsectionの配列だけを複製する。カテゴリ一覧とsectionに属する同一記事カードも`Arc`で共有する。JSONの文書構造は共有参照の有無に依存しない。

- local readerはdev-localと固定fixtureでfilesystem treeを読み、memory cacheを適用しない
- S3 readerはcurrent.jsonからrelease prefixを固定し、snapshotと取得済みartifactをmemory cacheする
- 公開前smokeではreleaseのsite prefixを直接指定し、current.jsonがない場合はそのprefixから同じ構造を読む

release snapshotのTTL・identity比較・stale fallbackはinfraが判断する。S3 snapshot cacheは既定5秒のTTLを持つ。TTL後もrelease identityが同じならartifact cacheを維持し、identityが変わったときだけ新cacheへ切り替える。既存requestが保持するsnapshotはrequest完了まで有効である。同じartifactへのconcurrent missは1回のreadへまとめ、errorはcacheしない。

snapshot更新に失敗した場合、cache済みの直前のimmutable releaseを配信し、次のTTLで再試行する。運用中にcurrent.jsonが消えても直接指定したprefixへ戻らない。初回取得失敗、直接指定したprefix、TTL=0ではstale fallbackしない。未取得artifactのread失敗まで隠さず、そのrequestはerrorにする。

artifact-backedなGET / HEADはprocess instance、release identity、request URIからweak ETag、release生成時刻とprocess起動時刻からLast-Modifiedを作る。URIを含めて異なるrepresentationのvalidatorを分離し、process再起動でもvalidatorを変えてserver / UI更新後の古いrepresentationを再利用させない。ETagの解析・weak比較はheaders crateに委譲する。wildcard単独では描画前の304を返さない。If-None-MatchをIf-Modified-Sinceより優先し、resourceが存在する成功responseだけを304へ変換する。

Topcoatのpage再描画はpage自身のURLにruntime header付きでPOSTし、runtime layerがapplication layerより前にGETへ変換するため、cache対象のmethod判定にはoriginal methodを使う。local reader、release prefixを直接読む公開前smoke、TTL=0、health / readiness、asset、404 / error responseにはartifact validatorを付けない。

/api/healthはprocess livenessだけ、/api/readyはconfigured readerからsnapshotとsite metadataを読めることを確認する。cache済みstale snapshotを配信できる場合はreadyとする。

/api/openapi.jsonは、記事一覧・health・readinessの3 APIをTopcoatの登録済みrouteとOperationからOpenAPI 3.1文書として生成する。本文schemaは公開artifact型から導出し、公開URLはBaseUrlを使う。生成時にartifactを読まず、content releaseのvalidatorは付けない。SSR・asset・内部runtime routeはAPI文書へ含めない。

## 配備と運用

コンテンツreleaseとapplication releaseは独立している。

- GitHub Actionsは公開MarkdownからS3 content releaseを作る
- VPS deployはserver binaryとTopcoat asset bundleを同じapplication releaseとして切り替える
- serverは127.0.0.1:8008だけで待ち受け、cloudflaredが外向きTunnelへ接続する
- productionのS3 readerはIAM Roles AnywhereのX.509 identityとcredential_processで期限付きcredentialを取得する

production runtimeへlong-lived IAM user key、credential file、application独自のcredential refreshを持ち込まない。S3 uploadもRust applicationの責務にしない。

mise run dev-localはGit管理済みの公開Markdownからlocal artifactを生成してserverを起動する。原文同期はsync-obsidian、抽出・翻訳はexportとして分離する。S3 readerの確認はdev / test-e2e-s3、外部状態に依存しないCI回帰は固定fixtureのtest-e2eを使う。

taskの正本は[mise.toml](../../mise.toml)、E2Eの実行方法は[e2e README](../../e2e/README.md)、systemdとruntime設定は[service README](../../service/README.md)、初期構築・更新・障害対応は[operations](../operations/README.md)に置く。

## 非目標

- DBによる記事管理
- 認証・認可、管理画面、ブラウザ編集
- マルチユーザー、SaaS CMS
- リアルタイム更新
- full-site search
- multiple bucket / prefixをまたぐ配信
- runtimeでのMarkdown変換
