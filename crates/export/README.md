# export

private Obsidianの公開対象だけを、Git管理できるMarkdownへ抽出するローカルコマンド。

```sh
mise run export-ja
# 明示的な入力・出力を使う場合
cargo run -p export -- --source /path/to/vault/Publish --output content
```

`is_completed: true` のノートだけを処理する。未知のfrontmatterはコピーせず、公開Markdown契約に必要なフィールドだけを書き出す。非公開ノートや公開ディレクトリ外のノートは参照先として取り込まない。symlink入力は拒否する。

`content/ja/<id>.md` に日本語を出力する。初回IDは従来のtitle／元path／createdによるslugを保持する。再実行では元pathのdigestで既存IDを探し、path変更時は削除された元pathと同じkind・createdを持つ候補が1つのときだけ引き継ぐ。曖昧なら原文へ `publish_id: <既存ID>` を指定する。category移動時にはcategoryを含むURLが変わるため、差分を確認する。

WikiLink・通常の内部Markdownリンクは `content:<id>` に正規化する。note embedは公開先へのリンクとして扱う。単独の見出し参照も含め、参照先は公開ノート集合内で解決する。未解決・曖昧なノート／見出しはエラーとする。見出しには原文から決定するanchorを付ける。PNG/JPEG/GIF/WebP/AVIFは参照されたファイルだけをcontent hash名で `content/assets/` へコピーする。ローカル参照を含むraw HTMLはMarkdownのリンク／画像へ書き直してから実行する。外部URLのbookmark HTMLは使用できる。

出力は同じfilesystemの一時ディレクトリで組み立て、検証成功後に入れ替える。同時exportはlockで拒否する。差分がなければ既存ファイルのmtimeも変えない。削除・非公開化された日英Markdownは `content/.export-archive/`（Git対象外）へ退避する。任意のREADME・隠しファイルは保持する。`ja/`、`en/` のMarkdownとhash名のassetはexport管理領域である。

中断で `.content.export-lock` が残った場合は、exportプロセスが終了したことを確認して削除する。`.content.export-backup` が残った場合、contentがなければbackupをcontentへ戻す。contentがある場合は新旧を比較して採用版を確定してからbackupを除去する。一時ディレクトリや退避版をGitへ追加しない。

公開Markdownはpush時点で公開されるため、commit前に差分を確認する。日本語の正本はObsidianであり、通常は公開版を直接編集しない。`publish`と`dev-local`は公開Markdownだけを入力にする。

## 翻訳

```sh
codex login
mise run export             # 原文抽出と英訳を同一transactionで実行
mise run translate          # 公開Markdownの英訳だけ。private入力は不要
cargo run -p export -- --translate-only --candidates
cargo run -p export -- --accept <id>
```

`translation/settings.json` のモデル・指示・用語集を使用する。必要なら `--settings PATH` で指定する。Codex CLI 0.154以降のChatGPTログインを使い、APIキー方式には切り替えない。実AIは通常テストでは呼ばない。

AI入力はpublic Markdownのtitle・summaryと、parserで抽出した文章fragmentだけ。コード・数式・HTML・リンク先は元のMarkdownに残し、翻訳された文章をescapeして元の位置に戻す。fragment分割をまたぐ大幅な語順変更は苦手なので、採用後の英語Markdownを必要に応じて手動編集する。raw HTML内の文言はこの処理では翻訳しない。用語集は該当する文章にだけ適用し、指定訳語と補間変数の保持を検証する。

実行時はpublic fragmentだけの一時workspaceを使い、Codexのfilesystem権限をminimal＋workspaceの読取に限定する。ユーザー設定・rules・AGENTSの読込、shell・apps・plugins・hooks・browser等のツールを無効にする。`read-only`だけでは全filesystemを読めるため、専用permission profileを指定する。CLIが設定を拒否した場合は停止する。認証・権限設定は[公式reference](https://learn.chatgpt.com/docs/config-file/config-reference)を参照する。

追跡情報は英語版frontmatterに保存する。入力hashは原文文章・Markdown構造・有効な用語集・モデル・指示・fragment方式versionを対象とし、日時・タグ等の管理情報は含めない。生成hashはtitle・summary・本文を対象とし、手動修正を検出する。AI応答のcacheは文章入力だけで決まるため、コードのみの更新では再翻訳せず新しい構造へ組み立て直す。

- 入力に変更がなければ、手動編集済みでも再利用する。
- 入力変更があり、最後の生成物から編集されていなければ更新する。
- 手動編集または履歴欠落があれば保護する。原文の更新時は更新待ちとして扱う。
- `--candidates` は保護された記事の候補を `.export-candidates/<id>.md` に作る。差分を確認し、`--accept <id>` で採用する。原文・設定が候補生成後に変わっていれば採用を拒否する。

失敗したrunは公開ファイルを入れ替えない。成功済みの応答だけをGit対象外の `.export-candidates/cache/` に保存するので、再実行で利用できる。候補・cache・退避版は公開しない。timeoutや利用上限ではエラーになり、従量課金へ自動切替しない。

## UIとタグの辞書

`mise run export`は記事とタグを抽出・翻訳した後、`crates/server/locales/ui.json`も更新する。`mise run translate`は公開Markdownと両辞書だけを使い、`mise run translate-ui`はUI辞書だけを処理する。UI辞書の形式・初期値・採用操作は[serverの説明](../server/locales/README.md)を参照する。

記事由来のタグは`content/tags.json`へ集約する。元のタグ文字列をキーにして、各項目に日本語の表示名`source`、用途`context`、英語の`translation.value`と生成履歴を保存する。同じタグを複数記事が使っても翻訳は一項目だけ。タグの対応はAIに渡さず、記事の`tags`は元のIDを保つ。不要になった項目は履歴を`.export-archive`へ退避して公開辞書から外す。

```sh
cargo run -p export -- --translate-only --candidates
cargo run -p export -- --accept-tag '統計'
```

タグ候補は`content/.export-candidates/catalog/`にキー・原文・用途付きで保存する。履歴付き訳文を手で修正し、その原文・用途・関連用語集・翻訳設定が変わると上書きを保護し`stale`にする。入力が同じなら手動訳を保持してAIを呼ばない。履歴がない値は自動生成とみなさず、保持して保護対象として報告する。欠落・更新待ちの英語タグは日本語表示名、辞書項目がなければ元のタグIDを使う。

記事・タグは一つのtransaction、UI辞書は別のtransactionで反映する。UIで失敗しても完了した公開コンテンツは保持される。再実行では成功済みの項目を再利用する。どちらも部分的に壊れたファイルを保存せず、最終的に両方の差分を確認してGitへ確定する。
