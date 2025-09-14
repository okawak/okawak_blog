---
created: 2025-05-17T15:54:30+09:00
updated: 2025-05-17T17:50:11+09:00
title: Obsidianのpluginを作成する(2)
category: tech
tags: [Obsidian, Obsidian_plugin, bun]
is_completed: false
priority: 2
summary: Obsidianの自作pluginを作成するメモで、まずは全体の構成を作成します。
---

# はじめに

前回の続きです。

- [[Obsidianのpluginを作成する(1)]]

今回は、公式サンプルではなく、自分で一から全体の構成を作ってみたいと思います。特にディレクトリ構成を設定しておき、ファイルの編集で拡張できるような枠組みを作成しておくのが今回の目的です。

文字列の処理などはRustを使って実装したいと思うので、Rustの設定もしておきます。

# ディレクトリ構造

プラグインの名前を`discord_message_sender`として、まずディレクトリを作成します。

```shell
cd /path/to/plugin_develop
cd .obsidian/plugins
mkdir discord_message_sender
cd discord_message_sender
```

## Gitの設定

Gitでコードを管理するのでまずは、Gitの設定をします。

```shell
git init
```

mainという名前のブランチで作成されるように設定を行なっています。公式サンプルの`.gitignore`を参考にtrackすべきでないファイルを記述します。

```shell
vi .gitignore
cat .gitignore
```

ファイルの内容は例えばこのような形です。(公式サンプルそのままです。)

```gitignore
# vscode
.vscode

# Intellij
*.iml
.idea

# npm
node_modules

# Don't include the compiled main.js file in the repo.
# They should be uploaded to GitHub releases instead.
main.js

# Exclude sourcemaps
*.map

# obsidian
data.json

# Exclude macOS Finder (System Explorer) View States
.DS_Store
```

## `bun`プロジェクトの設定

`npm`ではなく、今回は`bun`でプロジェクトを作成していくことにします。理由はなんとなく使ってみたかったからです。　

```shell
bun init
# blankを選択
```

すると、自動でファイルが作成され、`.gitignore`を除けば、以下のファイルが作成されると思います。とりあえず、出力されたままにしておきます。

```shell
.
├── bun.lock
├── index.ts
├── package.json
├── README.md
└── tsconfig.json
```

## Rustプロジェクトの初期化

クレート名として、`parse_message`としてプロジェクトを初期化します。

```shell
cargo new parse_message --lib
```

ここに必要な処理を書いていきます。また、`.gitignore`に`/target`を指定しておきます。

```shell
cat parse_message/.gitignore
# output
# "/target"
```

> はじめの"/"は「このディレクトリ以下の」という意味になります。

またweb assemblyとしてコンパイルするので、次の準備をしておきます。

```shell
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

## ディレクトリの整理

設定ファイルはルート直下におき、実装コードは`src/`の中に入れるようすると、隠しファイルを除いた最終的にディレクトリ構造はこのようになりました。

```shell
.
├── bun.lock
├── LICENSE
├── manifest.json
├── package.json
├── parse_message
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── src
│       └── lib.rs
├── README.md
├── vite.config.ts
├── src
│   └── main.ts
├── styles.css
└── tsconfig.json
```

ここで、勝手に追加したファイルの説明はこちらです。

- `LICENSE`: Githubにpushした後でMITライセンスを追加します。
- `manifest.json`、`styles.css`: Obsidianのpluginに必須のファイルですが、一旦空ファイルとしておき、後で設定します。
- `vite.config.ts`: 最終的に`main.js`にバンドルする時の設定ファイルです。
- `README.md`: 後で説明を書きます。

# まとめ

一旦ここまででやったことを振り返っておきます。

- Gitプロジェクトの作成
- Bunプロジェクトの初期化
- Rustプロジェクトの初期化
  - Web Assemblyの設定
- ディレクトリの整理(これがディレクトリ構造の正解かは分かりません。)

ここまではただの雛形で、ファイルの中身が適当です。次の記事で、具体的な実装を進めていきたいと思います。

[[Obsidianのpluginを作成する(3)]]
