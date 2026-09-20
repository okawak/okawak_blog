# 公開Markdownの契約

`domain::PublicContentMeta` はexportとpublish間で使用するschema version 1のfrontmatter契約である。Markdown本文の解析・ファイルI/Oは各consumerが所有する。`publish`はこの契約だけを入力にし、private vaultへアクセスしない。

- `schema_version: 1`、`id`、`locale: ja | en`、`kind: article | category | page | home` を必須とする。
- `title`、`created`、`updated`、元pathのSHA-256である `source_hash` を持つ。元path自体は公開しない。
- 記事・カテゴリは `category`、固定ページは `page: about` を持つ。homeはどちらも持たない。
- 記事だけが `tags` と `section_path` を持つ。`summary` と `priority` は任意。
- `id` はURLに使える安定識別子。consumerは公開Markdownのpathや翻訳タイトルから再計算しない。
- 翻訳追跡情報 `translation` は英語版だけに置き、`input_hash`、`generated_hash`、`stale` を持つ。digestはSHA-256の16進文字列。
- 未知のキーはdeserializationで拒否する。既知フィールドの組合せ、schema、日時、タイトル、digestは `validate()` で検証する。

`Locale::path()` は既存の日本語pathを保持し、英語には `/en` を付ける。artifact keyも日本語を保持し、英語に `en/` を付ける。未対応localeのparseは失敗する。UI側のfallbackはserverの責務である。

## タグ表示名

`content/tags.json`は記事由来タグの表示名を管理するschema v1の`LabelCatalog`。記事frontmatterの`tags`は元の文字列を安定IDとして保持する。exportが公開対象だけからIDを集約し、共通翻訳処理でラベルを更新する。各項目は`source`、`context`、任意の`translation`（`value` / `stale` / `provenance`）を持つ。詳細は[exportの操作手順](../../crates/export/README.md)を参照する。

publishは各言語で使用するタグだけを`site/tags.json` / `site/en/tags.json`へ`ID → 表示名`として保存する。英語の欠落・更新待ちは日本語名、項目欠落はIDにfallbackする。記事の索引内ではタグIDを変更しない。辞書が存在する場合のschema・補間変数・生成履歴の不正はpublishを停止する。旧公開入力で辞書がない場合はIDを表示名として扱う。
