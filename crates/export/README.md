# export

private Obsidianの公開対象だけをGit管理できるMarkdownへ抽出し、記事・タグ・UIの差分翻訳と更新候補の生成まで行うローカルコマンド。
リポジトリルートから実行し、入力は`obsidian/Publish`、出力は`content`、UI辞書は`crates/server/locales/ui.json`、翻訳設定は`translation.json`に固定する。

```sh
mise run export
```

通常実行にモード指定は不要。未翻訳を生成し、未変更の訳文を再利用する。原文更新時は未編集訳だけを更新し、手動訳は保持して更新候補を作る。具体的な確認・編集・採用の手順は[翻訳の運用](#翻訳)を参照する。private入力が読めなければ停止し、公開Markdownだけの処理へ自動で切り替えない。

通常実行は、記事・タグ・UIの処理段階、現在件数と総件数、生成・再利用・候補生成の別をINFOログへ表示する。AI呼び出し中は待機開始を表示し、10秒以上かかる場合は10秒ごとに経過時間を表示する。1回のAI呼び出しは最大300秒で、原文や翻訳結果はログへ出さない。

CLIは`clap`で引数を検証する。候補の採用は`accept-article <id>`、`accept-tag <id>`、`accept-ui <key>`で行う。pathを変更するオプションは設けず、通常実行と候補採用で同じ固定pathを使う。`--help`で対象ごとの使い方を確認できる。

`is_completed: true` のノートだけを処理する。未知のfrontmatterはコピーせず、公開Markdown契約に必要なフィールドだけを書き出す。非公開ノートや公開ディレクトリ外のノートは参照先として取り込まない。symlink入力は拒否する。

`content/ja/<id>.md` に日本語を出力する。初回IDは従来のtitle／元path／createdによるslugを保持する。再実行では元pathのdigestで既存IDを探し、path変更時は削除された元pathと同じkind・createdを持つ候補が1つのときだけ引き継ぐ。曖昧なら原文へ `publish_id: <既存ID>` を指定する。category移動時にはcategoryを含むURLが変わるため、差分を確認する。

WikiLink・通常の内部Markdownリンクは `content:<id>` に正規化する。通常Markdownのリンク・画像は参照元からの相対pathで解決し、同名の別ファイルへfallbackしない。WikiLink・Wiki embedだけは公開rootやbasenameからも候補を探す。note embedは公開先へのリンクとして扱う。単独の見出し参照も含め、参照先は公開ノート集合内で解決する。未解決・曖昧なノート／見出しはエラーとする。見出しには原文から決定するanchorを付ける。PNG/JPEG/GIF/WebP/AVIFは参照されたファイルだけをcontent hash名で `content/assets/` へコピーする。ローカル参照を含むraw HTMLはMarkdownのリンク／画像へ書き直してから実行する。外部URLのbookmark HTMLは使用できる。

出力は同じfilesystemの一時ディレクトリで組み立て、検証成功後に入れ替える。同時exportはlockで拒否する。差分がなければ既存ファイルのmtimeも変えない。削除・非公開化された日英Markdownは `content/.export-archive/`（Git対象外）へ退避する。任意のREADME・隠しファイルは保持する。`ja/`、`en/` のMarkdownとhash名のassetはexport管理領域である。

中断で `.content.export-lock` が残った場合は、exportプロセスが終了したことを確認して削除する。`.content.export-backup` が残った場合、contentがなければbackupをcontentへ戻す。contentがある場合は新旧を比較して採用版を確定してからbackupを除去する。一時ディレクトリや退避版をGitへ追加しない。

公開Markdownはpush時点で公開されるため、commit前に差分を確認する。日本語の正本はObsidianであり、通常は公開版を直接編集しない。`publish`と`dev-local`は公開Markdownだけを入力にする。

## 翻訳

```sh
codex login
mise run export
```

リポジトリルートの`translation.json`に記事・タグ・UI共通のモデル・翻訳指示・用語集を置く。コマンドはリポジトリルートから実行する。Codex CLI 0.154以降のChatGPTログインを使い、APIキー方式には切り替えない。実AIは通常テストでは呼ばない。

### 通常実行と候補採用の違い

`mise run export`は、公開対象の記事・タグ・UI辞書を順に処理する。毎回すべてを翻訳するのではなく、原文・翻訳設定と現在の訳文を確認し、必要な項目だけを生成・更新する。

「候補」は、手修正した訳文を自動更新で失わないよう、正式な訳文とは別のファイルに保存する新しい英訳。初回翻訳や、手修正していない英訳の更新は直接反映するため、毎回候補を確認・採用する必要はない。

| コマンド | 実行する処理 |
| --- | --- |
| `mise run export` | 原文を抽出し、記事・タグ・UI辞書を差分翻訳する。必要に応じて候補を作る |
| `cargo run -p export -- accept-article <記事ID>` | 指定した記事の候補だけを採用する。home・page・categoryも同じコマンドを使う |
| `cargo run -p export -- accept-tag '<元のタグ名>'` | 指定したタグの候補だけを採用する |
| `cargo run -p export -- accept-ui <キー>` | 指定したUI文言の候補だけを採用する |

`accept-*`を指定すると通常の抽出・翻訳処理は実行しない。保存済みの候補を使い、Obsidianの読込やAIの呼出しは行わない。候補が存在しない場合はエラーになり、その場で翻訳を生成することはない。

### 引数に指定する値

exportで確認が必要な候補があると、ログの`command`に対象を指定済みの採用コマンドが表示される。候補を確認・修正した後は、そのコマンドをコピーして実行できる。

自分でコマンドを入力する場合は、次の値を指定する。

| コマンド | 引数の確認場所 | 指定例 |
| --- | --- | --- |
| `accept-article` | `content/.export-candidates/`にある記事候補のファイル名から`.md`を除いた部分。公開Markdownのfrontmatterの`id`とも一致する | `012345abcdef.md`なら`012345abcdef` |
| `accept-tag` | `content/tags.json`の`entries`のキー。記事の`tags`に使っている元のタグ文字列で、候補JSONの`key`でも確認できる | `統計` |
| `accept-ui` | `crates/server/locales/ui.json`の`entries`のキー。候補JSONの`key`でも確認できる | `filter.label` |

例えば次のように指定する。記事IDやタグ名・UIキーは、確認した候補に合わせて置き換える。

```sh
cargo run -p export -- accept-article 012345abcdef
cargo run -p export -- accept-tag '統計'
cargo run -p export -- accept-ui filter.label
```

記事の引数はタイトルやファイルのpathではなくIDを渡す。タグの引数は翻訳後の表示名ではなく元のタグ文字列を渡し、空白を含む場合は`'Machine Learning'`のように引用符で囲む。タグ・UI候補のファイル名に付くhashは、どちらの引数にも使わない。

### 更新時に何が起きるか

ここでいう「翻訳入力」は、翻訳対象の原文やモデル・翻訳指示・関連する用語集など。記事ではMarkdown構造も含む。

| 現在の状態 | 通常のexportでの動作 |
| --- | --- |
| 英訳がまだない | 英訳を生成し、直接反映する |
| 生成履歴があり、翻訳入力に変更がない | 現在の英訳を再利用する。手修正があってもそのまま保持する |
| 翻訳入力が変わり、英訳は前回の生成時のまま | 新しい英訳を生成し、直接更新する |
| 翻訳入力が変わり、英訳は生成後に編集されている | 現在の英訳を保持し、新しい英訳を候補として保存する |
| 英訳はあるが生成履歴がない | 自動更新してよいと判断できないため、現在の英訳を保持して候補を作る |

現在の訳文を保護する場面で、同じ翻訳入力に対応する候補がすでにあれば、その候補を再利用する。候補を手修正していても、同じ入力での再実行では保持する。新しい訳文が必要な場合も、同じ要求のAI応答cacheがあれば再利用する。

### 候補の保存先と編集箇所

以下のpathはすべてリポジトリルートからの相対path。

| 対象 | 現在の訳文 | 候補ファイル | 候補で編集する箇所 |
| --- | --- | --- | --- |
| 記事 | `content/en/<id>.md` | `content/.export-candidates/<id>.md` | 英語のtitle・summary・本文 |
| タグ | `content/tags.json`内の対象entry | `content/.export-candidates/catalog/<hash>.json` | `translation.value` |
| UI辞書 | `crates/server/locales/ui.json`内の対象entry | `crates/server/locales/.export-candidates/catalog/<hash>.json` | `translation.value` |

タグ・UI候補のファイル名は、辞書ファイル名と対象キーから計算したhash。JSON内の`key`・`source`・`context`で対象と用途を確認する。採用コマンドにはhashではなく、タグIDまたはUIキーを渡す。

候補のID・locale・原文・用途・生成履歴（`translation`内のhashや`provenance`）は変更せず、訳文を編集する。`.export-candidates/`以下はGit管理対象外で、候補やcacheは公開しない。

### 記事の更新から候補採用まで

以下は記事IDが`article-1`の場合の例。実際のIDに置き換えて実行する。

1. 初回の`mise run export`で、日本語の公開用Markdownと英訳が作られる。英訳は`content/en/article-1.md`に直接保存される。
2. 必要に応じて、この英訳のtitle・summary・本文を手作業で修正する。生成履歴は残しておく。
3. 後日、Obsidian側の日本語を修正して`mise run export`する。exportは手修正した英訳を保持し、新しい原文に対応する英訳を`content/.export-candidates/article-1.md`へ保存する。ログには確認が必要な対象と採用コマンドが表示される。
4. 現在の英訳と候補を比較する。例えば次のコマンドで差分を確認できる（差分がある場合の終了コード1は正常）。

   ```sh
   git diff --no-index -- content/en/article-1.md content/.export-candidates/article-1.md
   ```

5. 必要なら候補ファイルの訳文を修正する。残したい既存の手修正も候補側へ取り込む。候補を採用すると、その内容で現在の英訳を置き換えるため、既存の手修正が自動でマージされることはない。
6. 候補の内容に問題がなければ、次を実行する。

   ```sh
   cargo run -p export -- accept-article article-1
   ```

7. 検証に成功すると、候補のtitle・summary・本文が`content/en/article-1.md`へ反映され、候補ファイルは削除される。`git diff`で公開用ファイルの変更を確認してからcommitする。

候補を採用するまでは、現在の英訳を保持する。候補を編集しただけでは正式な英訳に反映されない。タグ・UI辞書も、対象の候補JSONを確認・編集してから対応する`accept-*`を実行する。

### 採用時の検証と古い候補の扱い

採用時には、対象のID・キー、生成履歴、候補が現在の翻訳入力に対応しているかを確認する。記事の比較対象は`content/ja/<id>.md`と現在の翻訳設定で、Obsidian側だけにある未exportの変更は検知しない。日本語をさらに変更していた場合は、その更新をexportし、更新後の原文に対応する候補を確認してから採用する。

翻訳に関わる原文・設定が候補生成後に変わっていれば、古い候補の採用は拒否する。記事のカテゴリ・タグ・日時などの管理情報だけが変わった場合は、最新の公開日本語版からその情報を反映する。タグ・UIでは補間変数や必要な用語の保持なども検証する。これらは機械的な整合性確認であり、訳文の品質は人が確認する。

翻訳入力が変わった状態で、手修正済みの古い候補や生成履歴のない候補が残っていると、exportは候補を上書きせず停止する。その場合は古い候補をGit管理対象外の別の場所へ退避し、exportを再実行して新しい候補を作る。退避した候補と比較し、残したい修正を新しい候補へ取り込んでから採用する。

正式な英訳を削除した場合なども、同じ入力に対応する候補が残っていれば、自動生成せず停止する。候補を確認して採用するか、退避してからexportを再実行する。

### 手修正を判定する仕組み

AIが訳文を生成した時点で、実際の出力から`generated_hash`を計算して記録する。次回は現在の訳文からhashを計算し、その記録と比較する。一致すれば生成時のまま、異なれば生成後に編集されたものとして扱う。AIにもう一度翻訳させて比較する仕組みではないため、AI出力の揺れは手修正判定に影響しない。人の編集だけでなく、別ツールによる変更も検出対象になる。

記事の`generated_hash`はtitle・summary・本文が対象で、日時・タグなどの管理情報は含まない。翻訳入力を記録する`input_hash`とともに、英語版frontmatterの`translation`に保存する。タグ・UIでは訳文の`value`を対象にし、各entryの`translation.provenance`に記録する。

候補を手修正して採用した場合も、hashを手修正後の内容で書き換えず、AI生成時の記録を保持する。そのため、同じ翻訳入力では採用した訳文が再利用され、次に原文などが変わった際にも手修正として保護される。

### 翻訳処理の詳細

AI入力はpublic Markdownのtitle・summaryと、parserで抽出した文章fragmentだけ。コード・数式・HTML・リンク先は元のMarkdownに残し、翻訳された文章をescapeして元の位置に戻す。fragment分割をまたぐ大幅な語順変更は苦手なので、採用後の英語Markdownを必要に応じて手動編集する。raw HTML内の文言はこの処理では翻訳しない。用語集は該当する文章にだけ適用し、指定訳語と補間変数の保持を検証する。

実行時はpublic fragmentだけの一時workspaceを使い、Codexのfilesystem権限をminimal＋workspaceの読取に限定する。ユーザー設定・rules・AGENTSの読込、shell・apps・plugins・hooks・browser等のツールを無効にする。`read-only`だけでは全filesystemを読めるため、専用permission profileを指定する。CLIが設定を拒否した場合は停止する。認証・権限設定は[公式reference](https://learn.chatgpt.com/docs/config-file/config-reference)を参照する。

入力hashは原文文章・Markdown構造・有効な用語集・モデル・指示・fragment方式とMarkdown再構築のversionを対象とし、日時・タグ等の管理情報は含めない。AI応答のcacheは文章入力と翻訳設定で決まるため、コードやエスケープ方式のみの更新では再翻訳せず新しい構造へ組み立て直す。

各transactionは成功した場合だけ公開ファイルへ反映する。記事・タグとUI辞書のtransactionは分かれているため、UIで失敗しても完了した記事・タグの更新は保持される。成功済みの応答はGit対象外の`.export-candidates/cache/`に保存するので、再実行で利用できる。候補・cache・退避版は公開しない。timeoutや利用上限ではエラーになり、従量課金へ自動切替しない。

## UIとタグの辞書

`mise run export`は記事とタグを抽出・翻訳した後、`crates/server/locales/ui.json`も更新する。いずれも未変更の翻訳・候補を再利用し、手動訳を保持したまま必要な更新候補を自動生成する。UI辞書の形式・初期値・採用操作は[serverの説明](../server/locales/README.md)を参照する。

記事由来のタグは`content/tags.json`へ集約する。元のタグ文字列をキーにして、各項目に日本語の表示名`source`、用途`context`、英語の`translation.value`と生成履歴を保存する。同じタグを複数記事が使っても翻訳は一項目だけ。タグの対応はAIに渡さず、記事の`tags`は元のIDを保つ。不要になった項目は履歴を`.export-archive`へ退避して公開辞書から外す。

辞書の原文で補間変数を追加・削除・改名した場合も差分翻訳できる。生成後に未編集の訳は更新し、手動編集・履歴不明の訳は保持して`stale`にする。更新待ちの古い補間変数を持つ訳は表示に使わず原文へfallbackし、生成・候補採用・公開時には有効な訳の補間変数の一致を検証する。

```sh
mise run export
cargo run -p export -- accept-tag '統計'
```

タグ候補は`content/.export-candidates/catalog/`にキー・原文・用途付きで保存する。履歴付き訳文を手で修正し、その原文・用途・関連用語集・翻訳設定が変わると上書きを保護し`stale`にする。入力が同じなら手動訳を保持してAIを呼ばない。履歴がない値は自動生成とみなさず、保持して保護対象として報告する。欠落・更新待ちの英語タグは日本語表示名、辞書項目がなければ元のタグIDを使う。

UI・タグの候補自体にも同じ手動編集保護を適用する。同じ入力での再実行では候補を保持し、入力変更後に手動編集済み候補が残っていれば停止する。候補をGit対象外の別pathへ退避してから再生成する。

記事・タグは一つのtransaction、UI辞書は別のtransactionで反映する。UIで失敗しても完了した公開コンテンツは保持される。再実行では成功済みの項目を再利用する。どちらも部分的に壊れたファイルを保存せず、最終的に両方の差分を確認してGitへ確定する。

辞書単独の操作は親directoryを走査・入れ替えず、対象JSONを一時ファイルから置換する。親directoryのsymlinkを解決してから、同じ公開treeのexportとlockを共有する。辞書ファイル自体のsymlinkは拒否する。失敗時も完成した候補・cacheはGit対象外の作業領域に残り、辞書本体は全項目の検証成功後に確定する。

## 実装の構成

| Module | 責務 |
| --- | --- |
| `operation` | export・辞書翻訳・候補採用の各操作の入口、処理順、lock・transaction境界 |
| `vault` / `vault::normalize` | 公開対象と安定IDの決定、公開ノート参照・画像・本文の正規化 |
| `translation` | 共通の翻訳要求・検証・結果型と、翻訳・候補処理の内部API |
| `translation::articles` / `catalog` | 記事／辞書固有の計画作成、翻訳の生成・反映、候補の検証・反映 |
| `translation::plan` | 現在の訳と候補から、再利用・生成・候補生成・候補再利用を決める純粋な判定 |
| `translation::fragments` / `cache` / `codex` | 文章抽出・再構築、検証済み応答の保存、Codexプロセス実行 |
| `output` | 公開Markdown・辞書のI/O、削除記事の退避、metadata・asset・タグの同期 |
| `content` / `filesystem` | I/Oを持たない文書処理／schemaに依存しない走査・lock・staging |

正規化内の参照index、asset読込、本文編集は同じmodule内の型と関数で分担する。翻訳処理はpublic文書だけを扱い、原文adapterへ依存しない。CLIとcrate外API、公開schema、翻訳履歴・候補・cacheの形式はmodule構成から独立している。

通常の検証は`cargo test -p export --offline`で実行できる（依存crateの取得済み環境）。統合テストは一時directoryとfake translator / fake Codexを使い、private Obsidianや実AIを必要としない。
