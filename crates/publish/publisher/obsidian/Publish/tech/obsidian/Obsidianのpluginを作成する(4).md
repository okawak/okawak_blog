---
created: 2025-05-18T17:31:01+09:00
updated: 2025-07-12T23:00:08+09:00
title: Obsidianのpluginを作成する(4)
category: tech
tags: [Obsidian, Obsidian_plugin, Discord]
is_completed: false
priority: 4
summary: プラグイン作成の動作確認とコミュニティプラグインへのPRを出すところまで説明します。
---

# はじめに

前回の続きです。今回で一旦動く形に持っていくので、これで最終回です。

- [[Obsidianのpluginを作成する(1)]]
- [[Obsidianのpluginを作成する(2)]]
- [[Obsidianのpluginを作成する(3)]]

# Discordの設定

Discordのメッセージをobsidianに送るためには、Discord側でも準備が必要です。まず、Obsidianに送る用のDiscordサーバーを作っておいてください。

## 1. Botの作成

まず、[Discord Developer Portal](https://discord.com/developers/applications)にアクセスし、アプリケーションを作成します。アクセス後(元バージョンでは、)下画面のようなページが見られると思うので、右上の**New Application**ボタンから、新たなアプリケーションを作成します。

![image](https://d1fhrovvkiovx5.cloudfront.net/642c9b33b0d8250e770448b88d78e2c2.png)

アプリ名を入力するウインドウが表示されると思うので、自由な名前を設定します。これは後からでも変えられます。

無事に作成されると、設定画面に移ります。今回は**send_to_obsidian**という名前をつけています。自分用なので、Descriptionなどには自分が分かるような説明を書いておくと良いと思います。

OU

続いて、左のメニューバー(左ペイン)から「Bot」を選択し、「Message Content Intent」を有効化しておきます。これを有効化しないとメッセージ本文を受け取ることができないそうです。

![image](https://d1fhrovvkiovx5.cloudfront.net/d284d81647f3dbf52a040cc7a6aa1362.png)

最後にこの画面から、トークンを保管しておきます。トークンをコピーし忘れた場合は、この画面の中の「Reset Token」から新たに生成することができます。これはアクセスに重要な文字列なので、Githubにpushしてしまう等がないように、大切に保管しておいてください。Obsidianからの接続の際にも用います。

## 2. サーバーにbotを招待する

ボットは作っただけではなく、サーバーに招待して初めて使うことができます。

先ほどの画面の左ペインの「OAuth2」をクリックし、下にスクロールすることで「OAuth2 URL Generator」という項目が見えると思います。

- ![image](https://d1fhrovvkiovx5.cloudfront.net/02355b8d6747734b75ae7b9799203132.png)

この部分の「scopes」に対して

- bot

を有効にし、下にスクロールし、「bot permissions」に対して

- View Channels
- Send Messages
- Read Message History
- Add Reactions

を選択します。するとそのオプションに応じ、ページの一番下にURLが生成されるのでこれをコピーします。それをブラウザに貼り付けると、招待するサーバーを選択する画面になり、選択後自動でそのBotがそのサーバーに招待されます。

これで準備は完了です！

# 動作確認

# プルリクエストを出そう!

# まとめ

この手順は、ユーザー側にも行ってもらう手順になっています。なるべくユーザーが行う手順を少なくするために、リアルタイム性を無くして、Obsidianを開いた時にメッセージの処理が行われるようにしています。

最後に、(WebAssemblyを使わなければ、)意外と簡単にプラグイン作成できちゃうことがわかってもらえたかなと思います。Obsidian使い込んで、自分好みになるようにどんどんカスタマイズしていければと思います。

また初めてOSSにPRを出してみたというのはいい経験になったと思うので、今後も気になるものがあれば、積極的にチャレンジしていこうと思います。
