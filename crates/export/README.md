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

公開Markdownはpush時点で公開されるため、commit前に差分を確認する。日本語の正本はObsidianであり、通常は公開版を直接編集しない。この段階ではpublishと公開workflowは従来入力を使う。
