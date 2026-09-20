# export

private Obsidianの公開対象だけをGit管理できるMarkdownへ抽出し、記事・タグ・UIの差分翻訳と更新候補の生成まで行うローカルコマンド。
既定の入力はリポジトリルートの`obsidian/Publish`。

```sh
mise run export
# 明示的な入力・出力を使う場合
cargo run -p export -- --source /path/to/vault/Publish --output content
```

通常実行にモード指定は不要。未翻訳を生成し、未変更の訳文を再利用する。原文更新時は未編集訳だけを更新し、手動訳は保持して更新候補を作る。private入力が読めなければ停止し、公開Markdownだけの処理へ自動で切り替えない。

通常実行は、記事・タグ・UIの処理段階、現在件数と総件数、生成・再利用・候補生成の別をINFOログへ表示する。AI呼び出し中は待機開始を表示し、10秒以上かかる場合は10秒ごとに経過時間を表示する。1回のAI呼び出しは最大300秒で、原文や翻訳結果はログへ出さない。

CLIは`clap`で引数を検証する。通常実行のpath指定は`--source`、`--output`、`--ui-catalog`、`--settings`。候補の採用は`accept article <id>`、`accept tag <id>`、`accept ui <key>`で行い、path指定は各対象の後ろに置く（例: `accept article <id> --output /path/to/content --settings /path/to/translation.json`）。`--help`で対象ごとの使い方を確認できる。

`is_completed: true` のノートだけを処理する。未知のfrontmatterはコピーせず、公開Markdown契約に必要なフィールドだけを書き出す。非公開ノートや公開ディレクトリ外のノートは参照先として取り込まない。symlink入力は拒否する。

`content/ja/<id>.md` に日本語を出力する。初回IDは従来のtitle／元path／createdによるslugを保持する。再実行では元pathのdigestで既存IDを探し、path変更時は削除された元pathと同じkind・createdを持つ候補が1つのときだけ引き継ぐ。曖昧なら原文へ `publish_id: <既存ID>` を指定する。category移動時にはcategoryを含むURLが変わるため、差分を確認する。

WikiLink・通常の内部Markdownリンクは `content:<id>` に正規化する。通常Markdownのリンク・画像は参照元からの相対pathで解決し、同名の別ファイルへfallbackしない。WikiLink・Wiki embedだけは公開rootやbasenameからも候補を探す。note embedは公開先へのリンクとして扱う。単独の見出し参照も含め、参照先は公開ノート集合内で解決する。未解決・曖昧なノート／見出しはエラーとする。見出しには原文から決定するanchorを付ける。PNG/JPEG/GIF/WebP/AVIFは参照されたファイルだけをcontent hash名で `content/assets/` へコピーする。ローカル参照を含むraw HTMLはMarkdownのリンク／画像へ書き直してから実行する。外部URLのbookmark HTMLは使用できる。

出力は同じfilesystemの一時ディレクトリで組み立て、検証成功後に入れ替える。同時exportはlockで拒否する。差分がなければ既存ファイルのmtimeも変えない。削除・非公開化された日英Markdownは `content/.export-archive/`（Git対象外）へ退避する。任意のREADME・隠しファイルは保持する。`ja/`、`en/` のMarkdownとhash名のassetはexport管理領域である。

中断で `.content.export-lock` が残った場合は、exportプロセスが終了したことを確認して削除する。`.content.export-backup` が残った場合、contentがなければbackupをcontentへ戻す。contentがある場合は新旧を比較して採用版を確定してからbackupを除去する。一時ディレクトリや退避版をGitへ追加しない。

公開Markdownはpush時点で公開されるため、commit前に差分を確認する。日本語の正本はObsidianであり、通常は公開版を直接編集しない。`publish`と`dev-local`は公開Markdownだけを入力にする。

## 翻訳

```sh
codex login
mise run export             # 原文抽出と英訳を同一transactionで実行
cargo run -p export -- accept article <id>  # 更新候補を確認・修正した後に採用
```

リポジトリルートの`translation.json`に記事・タグ・UI共通のモデル・翻訳指示・用語集を置く。コマンドはリポジトリルートから実行する。別の設定は`--settings PATH`で指定する。Codex CLI 0.154以降のChatGPTログインを使い、APIキー方式には切り替えない。実AIは通常テストでは呼ばない。

AI入力はpublic Markdownのtitle・summaryと、parserで抽出した文章fragmentだけ。コード・数式・HTML・リンク先は元のMarkdownに残し、翻訳された文章をescapeして元の位置に戻す。fragment分割をまたぐ大幅な語順変更は苦手なので、採用後の英語Markdownを必要に応じて手動編集する。raw HTML内の文言はこの処理では翻訳しない。用語集は該当する文章にだけ適用し、指定訳語と補間変数の保持を検証する。

実行時はpublic fragmentだけの一時workspaceを使い、Codexのfilesystem権限をminimal＋workspaceの読取に限定する。ユーザー設定・rules・AGENTSの読込、shell・apps・plugins・hooks・browser等のツールを無効にする。`read-only`だけでは全filesystemを読めるため、専用permission profileを指定する。CLIが設定を拒否した場合は停止する。認証・権限設定は[公式reference](https://learn.chatgpt.com/docs/config-file/config-reference)を参照する。

追跡情報は英語版frontmatterに保存する。入力hashは原文文章・Markdown構造・有効な用語集・モデル・指示・fragment方式とMarkdown再構築のversionを対象とし、日時・タグ等の管理情報は含めない。生成hashはtitle・summary・本文を対象とし、手動修正を検出する。AI応答のcacheは文章入力と翻訳設定で決まるため、コードやエスケープ方式のみの更新では再翻訳せず新しい構造へ組み立て直す。

- 入力に変更がなければ、手動編集済みでも再利用する。
- 入力変更があり、最後の生成物から編集されていなければ更新する。
- 手動編集または履歴欠落があれば保護する。原文の更新時は更新待ちとして扱う。
- 保護された記事の候補は `.export-candidates/<id>.md` に自動生成する。差分を確認し、`accept article <id>`で採用する。採用にはprivate入力やAIを使わず、現在の公開原文と設定で検証する。原文の文章・Markdown構造や翻訳設定が候補生成後に変わっていれば採用を拒否する。カテゴリ・タグ・日時等の管理情報だけの変更は最新の日本語版から反映し、候補のtitle・summary・本文と生成履歴を保持する。

同じ入力の候補があれば、再実行でも手動編集を保持する。入力変更後の候補に手動編集がある場合や生成履歴がない場合は、候補を上書きせず停止する。古い候補をGit対象外の別pathへ退避してから再実行し、新しい候補と比較する。

失敗したrunは公開ファイルを入れ替えない。成功済みの応答だけをGit対象外の `.export-candidates/cache/` に保存するので、再実行で利用できる。候補・cache・退避版は公開しない。timeoutや利用上限ではエラーになり、従量課金へ自動切替しない。

## UIとタグの辞書

`mise run export`は記事とタグを抽出・翻訳した後、`crates/server/locales/ui.json`も更新する。いずれも未変更の翻訳・候補を再利用し、手動訳を保持したまま必要な更新候補を自動生成する。UI辞書の形式・初期値・採用操作は[serverの説明](../server/locales/README.md)を参照する。

記事由来のタグは`content/tags.json`へ集約する。元のタグ文字列をキーにして、各項目に日本語の表示名`source`、用途`context`、英語の`translation.value`と生成履歴を保存する。同じタグを複数記事が使っても翻訳は一項目だけ。タグの対応はAIに渡さず、記事の`tags`は元のIDを保つ。不要になった項目は履歴を`.export-archive`へ退避して公開辞書から外す。

辞書の原文で補間変数を追加・削除・改名した場合も差分翻訳できる。生成後に未編集の訳は更新し、手動編集・履歴不明の訳は保持して`stale`にする。更新待ちの古い補間変数を持つ訳は表示に使わず原文へfallbackし、生成・候補採用・公開時には有効な訳の補間変数の一致を検証する。

```sh
mise run export
cargo run -p export -- accept tag '統計'
```

タグ候補は`content/.export-candidates/catalog/`にキー・原文・用途付きで保存する。履歴付き訳文を手で修正し、その原文・用途・関連用語集・翻訳設定が変わると上書きを保護し`stale`にする。入力が同じなら手動訳を保持してAIを呼ばない。履歴がない値は自動生成とみなさず、保持して保護対象として報告する。欠落・更新待ちの英語タグは日本語表示名、辞書項目がなければ元のタグIDを使う。

UI・タグの候補自体にも同じ手動編集保護を適用する。同じ入力での再実行では候補を保持し、入力変更後に手動編集済み候補が残っていれば停止する。候補をGit対象外の別pathへ退避してから再生成する。

記事・タグは一つのtransaction、UI辞書は別のtransactionで反映する。UIで失敗しても完了した公開コンテンツは保持される。再実行では成功済みの項目を再利用する。どちらも部分的に壊れたファイルを保存せず、最終的に両方の差分を確認してGitへ確定する。

`--ui-catalog ui.json`のような相対pathも利用できる。辞書単独の操作は親directoryを走査・入れ替えず、対象JSONを一時ファイルから置換する。親directoryのsymlinkを解決してから、同じ公開treeのexportとlockを共有する。辞書ファイル自体のsymlinkは拒否する。失敗時も完成した候補・cacheはGit対象外の作業領域に残り、辞書本体は全項目の検証成功後に確定する。
