# UI文言カタログ

`ui.json`のキーと用途はserverが所有する。exportはこのデータだけを読み、Rust / Topcoatのコードを変更しない。カテゴリID、URL、CSS、ログは翻訳しない。

schema versionは1。`entries.<key>`は`source`（日本語）、`context`（用途）、任意の`translation`を持つ。英訳は`value`、`stale`、任意の`provenance`（`input_hash` / `generated_hash`）で管理する。補間は`{count}`等の名前と出現回数を保持する。件数のzero / one / manyはそれぞれ独立したキーにする。共有型はdomainの純粋なデータ契約であり、メッセージキーと翻訳文はdomainへ置かない。

初期英訳は実装と同時に用意した校正可能な値で、Codex CLIの生成履歴を偽装しないよう`provenance`を付けていない。この状態は履歴不明として保護・報告され、通常実行では訳文を保持してAIの更新候補を作る。候補を採用すると以後の差分判定に必要な履歴が付く。

```sh
mise run export
cargo run -p export -- accept-ui filter.label
```

候補はこのdirectoryの`.export-candidates/catalog/`にJSONで保存する。キー・原文・用途・訳文を確認し、必要なら候補の`translation.value`を修正して採用する。採用操作はprivate入力やAIを使わない。原文や設定が変わった古い候補は採用できない。候補・cacheはGit対象外。

通常実行との違い、手修正の判定、候補の確認・編集・採用の流れは[exportの翻訳運用](../../export/README.md#翻訳)を参照する。

`src/i18n.rs`の型付き`Message`がruntimeのキーを定義し、原文欠落・英訳欠落をCIで検出する。新しい文言を追加するときは辞書と型付きキーの両方を更新する。runtimeでは英訳の欠落・更新待ちは日本語へfallbackし、キーがない場合はキー文字列を表示してログへ記録する。

UI文言はserver binaryと一緒に配布する。記事のS3 uploadだけでUI辞書は更新されない。記事由来のタグ表示名は`content/tags.json`から記事と同じreleaseへ含めるため、新しいタグにserverの再デプロイは不要。
