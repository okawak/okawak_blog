---
created: 2025-05-17T14:57:59+09:00
updated: 2025-05-31T09:50:11+09:00
title: Obsidianのpluginを作成する(1)
category: tech
tags: [Obsidian, Obsidian_plugin, Discord]
is_completed: false
priority: 1
summary: Obsidianの自作プラグインを作成するメモで、まずサンプルプラグインの動作確認をした時の記録です。
---

# はじめに

Obsidianを使った別端末との連携について、`Git`プラグインが非常に便利です。Githubレポジトリを作成しておき、Obsidianを開くときに自動でpullされるようにすれば、常に最新のメモにアクセスできます。

一方で、スマホから情報を送りたいときに、スマホでGit操作をするのは少々面倒です。(画面を押すだけですが、仮にConflictとかが起きると…)細かい作業には向いていないと思います。

スマホでObsidianを開くときは、見る専用の使い方をしたいです。一方で、スマホからもメモを取りたいことがよくあると思います。がっつり文章を書くというよりは、出先で、後で見返したい一文や一言、URLなど簡単なメモを想定しています。

LINEから送信できるようなプラグインがすでに開発されているそうですが、開発者が管理するCloudflareサーバーを介しているところが少し気になった部分と、LINEよりDiscord派なので、Discordから送れるようにしたいということで開発することにしました。

<div class="bookmark">
  <a href="https://github.com/onikun94/line_to_obsidian">line_to_obsidian</a>
</div>

## 仕様の整理

どういうプラグインにしたいのかをはじめに整理しておきます。

- DiscordからメモをObsidianのVaultに送りたい
- Discordを介すものの、なるべくユーザーだけで完結させるようにしたい。
- 上の要素を実現させるためには、Discordのbot機能を使う必要がありそう(ユーザー側にbotを作成してもらう)
- 常にbotを常駐させることはしない
  - 「送信成功」などのリアルタイム応答を実装するのは難しそう
  - それを実現するには、Botを常駐させるために何らかのサーバーを常に立てておく = 開発者のサーバーを経由するということなので、LINE_to_obsidianと同じ思想になってしまう
- Obsidianを開いた時に、自動で同期されるようにする
- モバイルでは非対応にする(Git問題)
- メッセージを送った時ではなく、デスクトップのObsidianを開いた時に処理結果をDiscordに送信するようにする
- 独自プレフィックス(!hogeなどのコマンド)を用意し、plugin側で処理する
  - 随時機能を追加できるようにする

このような思想でプラグインを作っていきます。

# 開発準備

## Vaultの準備

まず、pluginを開発するための環境を整えます。すでに使っているValut名が表示されている左下の部分をクリックし、検証用の新しいVaultを作成します。

![image](https://d1fhrovvkiovx5.cloudfront.net/aa574326f49f4a8bb5a6b6e881d53453.png)

今回は、このようにplugin_developという名前をつけています。すると、作った初期段階でのディレクトリ構造はこのようになっていると思います。

```shell
.
├── .obsidian
│   ├── app.json
│   ├── appearance.json
│   ├── core-plugins.json
│   ├── graph.json
│   └── workspace.json
└── Welcome.md
```

pluginはこの`.obsidian`ディレクトリの中に`plugins`というディレクトリ内に大きく4つのファイルが置かれることになります。

```shell
├── .obsidian
│   ├── plugins
│   │   ├── plugin-name
│   │   │   ├── data.json
│   │   │   ├── main.js
│   │   │   ├── manifest.json
│   │   │   └── styles.css
...
```

- `data.json`: (optional) plugin設定が保存されたファイル
- `main.js`: 本体
- `manifest.json`: plugin情報が保存されたファイル
- `styles.css`: (optional) plugin用のスタイル

つまり、新しいpluginを作成するということは、これらのファイルを作ることがゴールとなります。

## サンプルコードを動かしてみる

[obsidian公式のページ](https://docs.obsidian.md/Plugins/Getting+started/Build+a+plugin)に基づいて、まずはサンプルプラグインを動かしてみます。

```shell
cd /path/to/plugin_develop
mkdir .obsidian/plugins
cd .obsidian/plugins
git clone https://github.com/obsidianmd/obsidian-sample-plugin.git
```

`npm`または`yarn`などを使ってパッケージをインストールします。公式ドキュメントでは`npm`を使っており、なんでも良いですが、筆者は`bun`を使いました。

```shell
cd obsidian-sample-plugin
bun install # パッケージのインストール <-> npm install
bun run build # tsのビルド <-> npm run build
```

ディレクトリ構造はこうなります。

```shell
.
├── bun.lock
├── esbuild.config.mjs
├── LICENSE
├── main.js
├── main.ts
├── manifest.json
├── package.json
├── README.md
├── styles.css
├── tsconfig.json
├── version-bump.mjs
└── versions.json
```

これでディレクトリ内に`main.js`が作られたことを確認できると思います。次にこのサンプルプログラムの読み込みを行います。新たにObsidianを起動し、設定からコミュニティプラグインのタブをクリックすると、**Sample Plugin**というものが新たに表示されていることが確認できると思います。

![image](https://d1fhrovvkiovx5.cloudfront.net/12d50c7c6df851b8690200ed86d21b5e.png)

公式サンプルをベースにしつつ、プラグインの作成を進めてみたいと思います。

# まとめ

この記事では、プラグインの大まかな構成、仕組みを書きました。ほとんど公式サイトに書かれていますが、次の記事では実際に新たなpluginを実装していく部分に触れたいと思います。

[[Obsidianのpluginを作成する(2)]]
