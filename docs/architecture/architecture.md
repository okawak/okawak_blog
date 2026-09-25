# okawak_blog アーキテクチャ

## 文書の役割

この文書は、現行システムの責務、依存方向、境界をまたぐ不変条件を定める。入力schemaの詳細は[Obsidian template](../content/obsidian-template.md)と[公開Markdown契約](../content/public-markdown.md)、crate内部の利用方法は各crateのREADME、本番操作は[operations](../operations/README.md)を正本とする。実装計画と完了済みの移行経緯はIssue / PRに残す。

okawak_blogは、Obsidianで書いたMarkdownを公開artifactへ変換し、Topcoat SSRで配信する静的コンテンツ公開基盤である。Markdown変換と公開可否の判定はビルド時に完了させ、runtimeはartifact読取、routing、metadata、UIへ集中する。

## システム概要

1. ローカルのexportがprivate Obsidianから公開対象を抽出・正規化する
2. 公開文章、タグ表示名、UI文言の必要な差分だけをローカルCodexで翻訳する
3. review済みの日英Markdownと辞書をGitへ確定する
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

~~~text
okawak_blog/
├── obsidian/          # private入力のgit submodule
├── content/           # export済みの公開Markdown・タグ辞書
├── translation.json   # 記事・タグ・UI共通の翻訳設定
├── crates/
│   ├── export/
│   ├── domain/
│   ├── publish/
│   ├── infra/
│   └── server/
├── e2e/
├── docs/
├── scripts/
├── service/
└── terraform/
~~~

obsidian/は外部入力であり、crateの内部資産にしない。公開入力のcontent/と翻訳設定もworkspace rootで管理し、変換処理とデータを分離する。

~~~mermaid
flowchart TB
    Export[crates/export] --> Domain[crates/domain]
    Publish[crates/publish] --> Domain
    Infra[crates/infra] --> Domain
    Server[crates/server] --> Domain
    Server --> Infra
~~~

exportとpublishはruntime依存ではない。serverはinfraをapplication compositionで利用するが、UI componentはstorageへ直接依存しない。

### crates/export

- private Obsidianの公開対象抽出、参照・embed・assetの正規化、安定IDの管理
- 記事・タグ・UIに共通する差分翻訳、手動編集の保護、更新候補の生成と採用
- public fragmentだけを渡すCodex実行境界と、出力全体を一括更新するfilesystem transaction
- private入力が読めない場合は停止し、公開Markdownだけの処理へ切り替えない

操作と翻訳状態の詳細は[export README](../../crates/export/README.md)を参照する。

### crates/domain

- 公開コンテンツ、artifact、release pointer、page documentの純粋な共有契約
- category、slug、locale、pathなどの値と検証規則
- artifactから表示用documentを組み立てる純粋ロジック
- I/O、async、AWS SDK、HTTP frameworkを持たない

### crates/publish

- 公開Markdownの読込、種別分類、安定IDによるlink解決、HTML変換、安全化、artifact生成
- bookmark enrichmentを外部境界として注入し、pipelineが処理順を所有する
- 全言語を一時領域で生成・検証した後だけ既存siteと入れ替える
- lib.rsはmodule宣言とre-exportに限定し、crate外APIはpublish entrypoint、bookmark enricher、PublishError / Resultに限定する
- path処理の対応環境はmacOSとLinuxとする

入力、出力、検証方法は[publish README](../../crates/publish/README.md)を参照する。

### crates/infra

- ArtifactReader / ArtifactSnapshotによるstorage非依存の読取境界
- local filesystemとS3のreader、release解決、cache、source設定
- vault読取、Markdown変換、S3 upload、HTTP response、UIを扱わない

### crates/server

- 単一のproduction server binary、Topcoat application、公開route、metadata、UI、asset、API
- src/app.rsをmodule_router!()のrootとし、app/のfile moduleをURL構造へ対応させる
- storage非依存のPageLoaderと、artifact readerを接続するArtifactPageLoader
- release-aware conditional GET、health / readiness、readerの生成と注入
- UI文言・カテゴリ表示名の辞書と、言語選択・切替

file単位の構成、Topcoat、styleの境界は[server README](../../crates/server/README.md)を参照する。

### E2Eと運用

e2e/はserverとartifact readerを通るbrowser E2Eを所有する。通常CIは固定fixtureを使い、private submodule、AI、AWSへ依存しない。実S3 smokeはローカル手動確認と公開workflowのgateに限定する。

scripts/tests/は運用スクリプト、service/tests/はsystemd unitを検証し、mise run test-serviceでまとめて実行する。terraform/は通常のrepository作業では読み取り専用とする。

## コンテンツ境界

private Obsidianは執筆形式、content/はexportとpublish間の公開契約である。Obsidian固有frontmatter、WikiLink、embed、private pathをpublishへ渡さない。公開Markdownはschema version、安定ID、locale、kind、日時、元pathのdigestを持ち、英語版だけが翻訳追跡情報を持つ。正確なfieldと組合せは[公開Markdown契約](../content/public-markdown.md)を正本とする。

content kindはarticle、category、page、homeの4種類である。articleはcategoryと同名の入力directoryに置き、categoryからの相対pathをsection_pathとして一覧のgroupingに使う。URLはsection_pathに依存しない。執筆例とdirectory規則は[Obsidian template](../content/obsidian-template.md)に置く。

記事と内部linkは翻訳後も変わらないcontent IDで対応づける。タイトル、元file path、翻訳結果から公開identityを再計算しない。publishは対象言語のcontentが配信可能なら同じ言語、なければ日本語のURLへ解決する。未解決参照、曖昧な参照、公開対象外への参照はexportまたはpublishで失敗させる。

タグIDは記事frontmatterに保持し、表示名はcontent/tags.jsonから同じcontent releaseへ含める。UI文言とカテゴリ表示名はserver辞書で管理し、server binaryと一緒に配備する。domainは表示文言を持たず、識別子と共有schemaだけを所有する。

## Artifactとrelease

publishは次のsite treeを生成する。既存URLとの互換性のため、日本語はroot、英語はen/に置く。

~~~text
site/
├── locales.json
├── content-assets/
├── articles/
│   ├── index.json
│   └── <category>/<slug>.html
├── categories/<category>.json
├── pages/<page>.json
├── home.json
├── tags.json
├── metadata/site.json
└── en/
    ├── articles/
    ├── categories/
    ├── pages/
    ├── home.json
    ├── tags.json
    └── metadata/site.json
~~~

locales.jsonは同じreleaseで実在するpathとlocaleの対応、content-assets/は言語間で共有するcontent hash付きassetを持つ。各言語のmetadataはその言語で配信する記事だけを集計する。

home.jsonはhome fragmentがある場合だけ生成する。en/ subtreeも配信可能な英語contentがある場合だけ生成する。

日本語は記事1件以上、About、記事が属するcategory landingを必須とする。英語は翻訳履歴があり更新待ちでないcontentだけを採用し、landingがないcategoryの記事を掲載しない。英語Aboutは任意である。全言語の生成とpage document構築に成功した場合だけsiteを入れ替え、古いfileを残さない。

本番は次のimmutable release構造を使う。

~~~text
current.json
releases/
└── <release-id>/
    ├── manifest.json
    └── site/
~~~

current.jsonとmanifest.jsonは同じversion付きpointer schemaを使い、release ID、artifact prefix、publisher / source / content commit、生成時刻を保持する。content_commitを持たない旧releaseと、source_commitがprivate vaultを示す旧値をreaderで受け入れるのは後方互換のためである。新releaseのsource / content commitは公開repositoryの同じcommitを示す。

公開workflowはmainから手動実行し、次の順序を守る。

1. 対象commitが最新mainであり、push起因CIが成功済みであることを確認する
2. Git管理済みの公開入力からsiteを生成・検証する
3. 新しいrelease prefixへuploadし、object集合と各言語の表示を検証する
4. 対象commitがまだ最新mainであることを再確認する
5. current.jsonを最後に切り替える

releaseは上書きせず、公開runをrepository単位で直列化する。失敗したrunは新releaseを残してもpointerを変更しない。この順序により、readerは不完全なuploadを選ばず、遅いrunが公開pointerを古いcommitへ戻すこともない。

## 公開URLと言語

日本語URLは既存pathを維持し、英語だけ/en prefixを付ける。

| Page | 日本語 | 英語 |
| --- | --- | --- |
| Home | / | /en |
| About | /about | /en/about |
| Category | /:category | /en/:category |
| Article | /:category/:slug | /en/:category/:slug |

/articles/:slugと/categories/:categoryは現行の公開routeではない。末尾slashはqueryを維持した308で正規URLへ移す。

トップページのGET / HEADだけは、保存済みlocale cookie、Accept-Language、英語の順で初期言語を選ぶ。記事などの直接URLはブラウザ設定で変更しない。言語切替は同じpageの翻訳、対象言語homeの順に移動し、対象言語が公開されていなければ無効にする。選択はhost限定、Path=/、HttpOnly、SameSite=Laxのcookieへ1年間保存する。

同じpageの翻訳が実在する場合だけhreflangを出す。英語artifactがないURLは英語404とし、日本語本文を英語版として配信しない。英語UI文言の欠落・更新待ちは日本語、未知のUI keyはkey文字列、英語タグ表示名の欠落は日本語名へfallbackする。

## Pageとruntime境界

domainのpage documentは言語やstorageに依存しない表示データを表す。serverのpage componentはPageLoaderから、page document、タグ表示名、言語別pathをまとめたpresentationを受け取る。ArtifactPageLoaderだけがinfraのsnapshotを読み、local / S3 readerをUIへ持ち込まない。

1 request内のpage documentとmetadataは同じartifact snapshotから組み立てる。home fragmentだけを任意とし、必要なartifactの欠落・読取失敗を空値へ変換しない。404とstorage errorはroute境界で異なるstatusとして扱う。/api/articlesは互換endpointとして維持し、page専用APIは増やさない。

公開routeはTopcoatのmodule treeから導出する。app.rsをrootとし、static segmentとdynamic segmentをfile moduleでURL構造へ対応させ、mod.rsは使わない。日本語と英語は描画処理を共有する。release-aware HTTP cacheはroute固有処理ではなくglobal layerとして構成する。

Topcoat runtimeはresponsive menuとcategory内の記事絞り込みに使う。初期HTMLには全記事と通常linkを含め、JavaScriptや外部CDNが失敗しても本文とnavigationを利用可能にする。UI component、asset、styleの詳細は[server README](../../crates/server/README.md)を正本とする。

## Reader、cache、HTTP

ArtifactReaderは1処理で使うArtifactSnapshotを取得し、snapshotは同じreleaseのindex、metadata、HTML、辞書、assetを読む。

- local readerはdev-localと固定fixtureでfilesystem treeを読み、memory cacheを適用しない
- S3 readerはcurrent.jsonからrelease prefixを固定し、snapshotと取得済みartifactをmemory cacheする
- current.jsonが初回から存在しない場合だけ旧bucket rootを日本語releaseとして読む

S3 snapshot cacheは既定5秒のTTLを持つ。TTL後もrelease identityが同じならartifact cacheを維持し、identityが変わったときだけ新cacheへ切り替える。既存requestが保持するsnapshotはrequest完了まで有効である。同じartifactへのconcurrent missは1回のreadへまとめ、errorはcacheしない。

snapshot更新に失敗した場合、cache済みの直前のimmutable releaseを配信し、次のTTLで再試行する。運用中にcurrent.jsonが消えてもlegacy rootへ戻らない。初回取得失敗、legacy root、TTL=0ではstale fallbackしない。未取得artifactのread失敗まで隠さず、そのrequestはerrorにする。

artifact-backedなGET / HEADはprocess instance、release identity、request URIからweak ETag、release生成時刻とprocess起動時刻からLast-Modifiedを作る。URIを含めて異なるrepresentationのvalidatorを分離し、process再起動でもvalidatorを変えてserver / UI更新後の古いrepresentationを再利用させない。If-None-MatchをIf-Modified-Sinceより優先し、resourceが存在する成功responseだけを304へ変換する。

Topcoatのpage再描画はpage自身のURLにruntime header付きでPOSTし、runtime layerがapplication layerより前にGETへ変換するため、cache対象のmethod判定にはoriginal methodを使う。local reader、legacy root、release prefixを直接読む公開前smoke、TTL=0、health / readiness、asset、404 / error responseにはartifact validatorを付けない。

/api/healthはprocess livenessだけ、/api/readyはconfigured readerからsnapshotとsite metadataを読めることを確認する。cache済みstale snapshotを配信できる場合はreadyとする。

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
