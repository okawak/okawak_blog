---
created: 2025-05-17T17:48:24+09:00
updated: 2025-05-18T09:56:47+09:00
title: Obsidianのpluginを作成する(3)
category: tech
tags: [Obsidian, Obsidian_plugin, Rust]
is_completed: false
priority: 3
summary: プラグインの実装部分についての簡単な記録です。
---

# はじめに

前回の続きです。

- [[Obsidianのpluginを作成する(1)]]
- [[Obsidianのpluginを作成する(2)]]

具体的な実装を書き、プラグインとして必要最低限の機能を作ります。Discord自体の設定も必要なので、動作確認は次の記事で行おうと思います。

# いざ実装へ！

実装といっても詳細は書きません。どういった流れでプラグインが作成できるかということに着目し、詳しい動作はコードを見て確認してください。(おそらく、細かい部分は今後修正を行うと思うので、枠組みのみを説明します。)

特にRustとTypeScriptの連携の部分を重点的に説明しようと思います。

## Rust

Rustはただの趣味です。Rustじゃなくても良いですし、むしろTypescriptに統一したほうが見やすいと思いますが、使いたかったので使います。以下`parse_message`ディレクトリ(Cargoプロジェクト下)での操作とします。

まず、WebAssemblyにコンパイルできるようにするため、`Cargo.toml`にlibと依存関係の設定を追加します。

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
```

`wasm-bindgen`を追加するときは、コマンドラインからでも問題ないです。

```shell
cargo add wasm-bindgen
```

`src/lib.rs`には、単純な処理だけを書いておき、後から拡張できるようにします。Stringを返すだけの関数を定義します。

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn to_md(content: &str) -> String {
    format!("{}\n", content)
}
```

これを`web-pack`でビルドします。

```shell
wasm-pack build --release --target web -d ../pkg
```

> targetに指定するものは、今回はwebを選択しています。場合に応じて適切なものを選択してください。

出力されるディレクトリはルートディレクトリ下の`pkg`となります。このコマンドはルートディレクトリの`package.json`のscriptで定義して`bun run wasm`でビルドできるようにします。(ルードでビルドした時は、ルートに`pkg`ディレクトリが作成されるようにします。)

```json
"scripts": {
  "wasm": "wasm-pack build parse_message --release --target web -d ../pkg",
}
```

この時、pkgはビルドした時に生成されるものなので、`.gitignore`に設定しておきます。

## Typescript

pluginの肝となる部分です。

# まとめ
