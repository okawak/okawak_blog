# publish

Git管理した公開Markdownから配信用HTML／JSONを生成する。private Obsidian・AI・S3 uploadは扱わない。

```sh
cargo run -p publish
cargo run -p publish -- --input content --output crates/publish/dist
mise run dev-local
```

入力は `content/ja/<id>.md`、任意の `content/en/<id>.md` と参照asset。正確なfrontmatterは [公開Markdown契約](../../docs/content/public-markdown.md) を参照する。執筆用Markdownは先に [export](../export/README.md) で抽出する。`is_completed`やWikiLinkを含むObsidian形式は直接受け付けない。

日本語は既存の `site/articles/`、`site/categories/`、`site/pages/`、`site/home.json`、`site/metadata/site.json` を生成し、英語は同じ構造を `site/en/` に生成する。各言語の集計は、その言語で公開する記事の件数である。`site/locales.json` は配信可能なpathの言語対応表であり、参照画像だけを `site/content-assets/` に配置する。

日本語は記事1件以上・About・記事カテゴリのlandingを必須にする。英語は翻訳履歴があり更新待ちでない版だけを採用し、未翻訳landingのカテゴリの記事は掲載しない。英語Aboutは任意。`content:<id>#<anchor>` は英訳が配信対象なら `/en/...`、そうでなければ日本語URLへ解決する。未解決ID、未知のschema、identity不整合、未正規化参照はエラーにする。

全言語を一時ディレクトリで生成・検証してからsiteを入れ替える。失敗時は既存siteを保持し、成功時は削除された記事のHTMLも消える。中断で `dist/.site-backup` が残った場合は、新旧siteを比較して復旧してから再実行する。

本文はpulldown-cmarkのevent pipelineでHTMLに変換し、URLとraw HTMLを安全化する。数式とコードは既存の描画を維持する。exportが作る空の見出しanchorと、従来のsimple bookmark構文だけをraw HTMLとして許可する。bookmarkは既存のOGP取得処理で拡張する。

```html
<div class="bookmark">
  <a href="https://example.com">Example</a>
</div>
```

`lib.rs`はmodule宣言と公開APIのre-exportのみ。`pipeline`が処理順、`input`が公開契約の読込、`classify`が描画用の種別分割、`links`が言語別URL解決、`render`がHTML変換、`artifacts`が出力を担当する。公開APIはpublish entrypoint、bookmark enricher注入、`PublishError`／`Result`に限定する。

通常テストは公開fixtureとfake bookmark enricherを使う。`cargo test -p publish` で実行でき、private入力・AI・AWSは不要。
