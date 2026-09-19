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

この文書はデータ契約を記載する。移行計画と進捗は [#261](https://github.com/okawak/okawak_blog/issues/261) を参照する。
