---
title: CodeQLの対象にActions追加、bun.lock登場など｜Productivity Weekly(2024-12-18)
source: "https://zenn.dev/cybozu_ept/articles/productivity-weekly-20241218"
author:
  - "[[Zenn]]"
published: 2024-12-31
created: 2025-07-20
description:
tags: ["clippings"]
updated: 2025-07-20T11:22:55+09:00
---

9[idea](https://zenn.dev/tech-or-idea)

こんにちは。サイボウズ株式会社 [生産性向上チーム](https://www.docswell.com/s/cybozu-tech/5R2X3N-engineering-productivity-team-recruitment-information) の平木場です。

僕たち生産性向上チームは毎週水曜日に Productivity Weekly という「1 週間の間に発見された開発者の生産性向上に関するネタを共有する会」を社内で開催しています。

本記事はその時のネタをまとめたものです。

2023-01-25 号から、基本的に隔週で連載することとしました。たまに単独でも投稿するかもしれません。

今週は 2024-12-18 単独号です。

今回が第 172 回目です。過去の記事は。

# News 📺

# Find and Fix Actions Workflows Vulnerabilities with Codeql (Public Preview) - Github Changelog

GitHub Code scanning の CodeQL において、GitHub Actions のワークフローがサポートされました（public preview）。

CodeQL はコードを走査して脆弱性につながる記述を教えてくれる静的解析ツールです。これまで JS/TS や Go などがサポートされていましたが、今回 GitHub Actions のワークフローもサポートされました。

利用するには各リポジトリの CodeQL 設定で `GitHub Actions` を有効化する必要があります。CodeQL をすでに有効化している場合でも改めて設定が必要です。たくさんリポジトリを持っていると面倒ですね。

試しに自分のホームページを管理しているリポジトリで有効化してみました。怒られすぎ。

![](https://res.cloudinary.com/zenn/image/fetch/s--420n0cUi--/c_limit%2Cf_auto%2Cfl_progressive%2Cq_auto%2Cw_1200/https://storage.googleapis.com/zenn-user-upload/deployed-images/d428ecfb53a29abe7467ff05.png%3Fsha%3D9a6a64ae51f43c0a1886574ed0f34386c6bd2fad)  
*一覧画面*

![](https://res.cloudinary.com/zenn/image/fetch/s--ZkaKOpNg--/c_limit%2Cf_auto%2Cfl_progressive%2Cq_auto%2Cw_1200/https://storage.googleapis.com/zenn-user-upload/deployed-images/977849fb741431b0b5077180.png%3Fsha%3D6a4c5572eff68f53b62b84e93db66014003a31f8)  
*詳細の一つ*

permission をしっかり設定しろとか、non-immutable なアクションの指定はやめろとか当たり前だけど大事なことを言っていますね。（permission に関してはリポジトリのデフォルトを read にしているから設定していなかった。google-github-action はめんどくてやれていなかった…）

みなさんも有効化してみてはいかがでしょうか。もちろん private/internal リポジトリにおいては GitHub Advanced Security が必要で有料です。

*本項の執筆者: [@korosuke613](https://zenn.dev/korosuke613)*

# Github Issues & Projects–Close Issue as a Duplicate, Rest Api for Sub-issues, and More! - Github Changelog

GitHub Issues、Projects において、Issue を重複（duplicate）扱いにしてクローズできるようになりました。これまで重複であることを明示的に示したい場合は label を付与するなどのユーザー側の工夫が必要でした。

これにより、なぜクローズされたかがますます分かりやすくなりましたね。

なお、上記記事では他のアップデートも書かれています。

- Sub Issue に関する REST API が利用可能に
- Sub Issue と Issue Type の制限値が緩和
	- Parent Issue ごとに最大 100 件の Sub Issue を付与可能に（+50 件）
	- Organization 内で利用できる Issue Type が最大 25 件に（+15 件）
- GitHub Mobile で Issue Type の表示、追加、更新が可能に
- Sub Issue と Issue Type のフィルタリングに `has:` と `no:` が使えるように
- その他もろもろ

GitHub の Issue 管理がますます柔軟にできるようになってきましたね。活用していきたいです。

*本項の執筆者: [@korosuke613](https://zenn.dev/korosuke613)*

# Bun's New Text-based Lockfile | Bun Blog

Bun のロックファイルをテキスト形式で扱えるようになりました 🎉

可愛いキャラクターでお馴染みの JavaScript ランタイムである Bun ですが、`bun install` コマンドを使ってインストールしたパッケージの情報を記載した `bun.lockb` ファイルはバイナリ形式で書かれており、プルリクエストのレビューがしづらい問題や、コンフリクト発生時に解決しにくい問題など、いくつか可愛くない問題を抱えていました。

しかし今回新たに `--save-text-lockfile` オプションが登場し、このオプションを使うことでロックファイルを bun.lock というテキスト形式のファイルで表現することが可能になりました！

こちらが実際の中身の様子です。json 形式で書かれているので、`package-lock.json` を扱っていた人にとっては今までに近い見た目で嬉しいですね。

```json
{
  "lockfileVersion": 0,
  "workspaces": {
    "": {
      "dependencies": {
        "uWebSocket.js": "uNetworking/uWebSockets.js#v20.51.0",
      },
    },
  },
  "packages": {
    "uWebSocket.js": ["uWebSockets.js@github:uNetworking/uWebSockets.js#6609a88", {}, "uNetworking-uWebSockets.js-6609a88"],
  }
}
```

`bun install --save-text-lockfile` コマンドは `bun.lockb` や `package-lock.json` がある場合はそちらに書かれた内容をもとにテキストファイルを生成してくれます。npm や yarn などの他のパッケージマネージャーからの移行もスムーズになりそうです。

この機能は 2024 年 12 月 17 日にリリースされた Bun v.1.1.39 から使用でき、Bun v1.2 からはテキストファイルでのロックファイル管理をデフォルトにすることを計画しているようです。

*本項の執筆者: [@takamin55](https://zenn.dev/takamin55)*

# Go Protobuf: the New Opaque Api - the Go Programming Language

Go の Protocol Buffers 実装である [google.golang.org/protobuf](https://pkg.go.dev/google.golang.org/protobuf) で、従来よりも効率的なコード生成を行うための新しい API が導入されました。

従来の実装では、Protocol Buffers 上の message に対応する Go の構造体が生成され、プログラマはその構造体のフィールドを直接自由に操作できました。

新しい Opaque API では、構造体の各フィールドはエクスポートされず、代わりに `GetFoo()` や `SetHoo()` などのアクセサメソッドを介してのみアクセスできます。

Opaque API では構造体の詳細が隠蔽されるため、bit field や遅延読み込みなどの最適化が可能になります。

また、アクセサメソッドを介することで、誤った使い方を防ぐ狙いもあるようです。

従来の API は引き続き利用可能で、移行の必要はありませんが、新規開発では Opaque API を利用することが推奨されています。

従来の API と Opaque API の間を取り持つ Hybrid API も提供されており、これを用いた [移行ツールと移行ガイド](https://protobuf.dev/reference/go/opaque-migration/) も公開されています。

*本項の執筆者: [@takoeight0821](https://zenn.dev/takoeight0821)*

# Know-how 🎓

# Storesのawsルートユーザーを全部削除しました - Stores Product Blog

2024 年 11 月に登場した Root access management を使うことで、AWS ルートユーザーしかできなかった操作の一部が他のユーザーでできるようになりました。この記事では、AWS Organizations 下の AWS ルートユーザー全てを削除（正確にはルートユーザーの認証情報を削除）したこと、およびそこまでの意思決定の様子が書かれています。

これによりセキュリティ向上と、ルートユーザーの認証情報の管理の煩雑さからの解放というメリットが得られたとのことです。

ただし、Security Hub で警告が出るようになってしまうとのこと。ルートユーザーには MFA が設定されているべきですが、今回の認証情報を削除したことでこのルールに抵触してしまうそうです。これは AWS 側の対応が待たれますね。

認証情報削除などの具体的な手順については記事に書かれていません。DeveloperIO さんの記事が詳しいので、そちらをご覧になると良いかもしれません。

- [待望！管理アカウントでメンバーアカウントのルートユーザ操作禁止などが設定できるRoot access managementがリリースされました！ | DevelopersIO](https://dev.classmethod.jp/articles/root-access-management/)

*本項の執筆者: [@defaultcf](https://zenn.dev/defaultcf)*

# Nilaway による静的解析で「10 億ドル」を節約する #kyotogo / Kyoto Go 56th - Speaker Deck

Go プログラムに `nil` が存在しないか検査する静的解析ツール NilAway の紹介スライドです。

[以前に Productivity Weekly でも紹介した](https://zenn.dev/cybozu_ept/articles/productivity-weekly-20241030#go%E3%81%AEnil-panic%E3%82%92%E9%98%B2%E3%81%90%E9%9D%99%E7%9A%84%E8%A7%A3%E6%9E%90%E3%83%84%E3%83%BC%E3%83%AB%EF%BC%9Anilaway) NilAway 再来です。

実は社内の LT 会で似た内容を喋ろうと考えていたので、先を越された……！という気持ちです。10 億ドルのくだりも同じで草。アルゴリズムの詳細でも喋ろうかな……。

*本項の執筆者: [@ajfAfg](https://zenn.dev/arjef)*

# Github Actionsのガードを高くする - Techouse Developers Blog

Techouse さんによる GitHub Actions のセキュリティリスクを減らすガード策の紹介記事です。

GitHub Actions は便利で自由度が高い反面、さまざまなセキュリティリスクが存在します。この記事では、セキュリティリスクごとのリスクを軽減できるツールと、それらを用いてワークフローを監査するワークフロー例が紹介されています。

取り上げられているセキュリティリスクは次の 3 つです。

- 使用している action が汚染されないか
- GITHUB\_TOKEN の権限が広く設定されていないか
- シェルスクリプトの実装ミスがないか

それぞれなぜリスクたり得るのかと対策ツールの何が嬉しいのかが説明されており、GitHub Actions のセキュリティを高めたい場合に参考になると思います。また、対抗策を適用できるすぐに使えるワークフローも MIT ライセンスで用意されているので、どのように実践すれば良いのかすぐにわかることも嬉しいですね。

皆さんも GitHub Actions のセキュリティ意識を高めて安心安全な開発ライフを送っていきましょう。

*本項の執筆者: [@korosuke613](https://zenn.dev/korosuke613)*

# 自作キーボードのキーマップ最適化のためにキー入力分析基盤を作ってみた（前編）

分割キーボード Keyball44 のキーマップを、実際の打鍵データに基づいて可視化するためにデータ収集・分析・可視化の基盤を作ったそうです。

これは前半として、PC でローカルに動作するキーロガーの実装を紹介されています。

キー入力を ELK スタック(Elasticsearch, Logstash, Kibana)でリアルタイムに Kibana に反映されるようにしていて、ヒートマップのような図が面白いです。バックスペース、エンター、母音が多いのは納得ですね。

DuckDB を使ったらどうなるだろうかとか、キーロガーをキーボード側に実装できないだろうかとか、色々興味が湧きました。僕も自分の持っている分割キーボードでできないか調べてみます。

*本項の執筆者: [@uta8a](https://zenn.dev/uta8a)*

# Tool 🔨

# ワークフローの完了をローカルに通知する Github Cli 拡張機能を作りました

ワークフローの完了をローカルで音と通知で知らせてくれる Mac 向けの GitHub CLI 拡張機能を作ったそうです。

実際に動かしてみました。

![gh-prowl-waiting](https://res.cloudinary.com/zenn/image/fetch/s--LBjGtc3y--/c_limit%2Cf_auto%2Cfl_progressive%2Cq_auto%2Cw_1200/https://storage.googleapis.com/zenn-user-upload/deployed-images/95f581d5f3137eec546540ee.png%3Fsha%3D0fa4486d3af6855bf969fd0eaf9c8f8f9133902a)  
*gh prowl コマンドを打って待っている間の画面*

![gh-prowl-done](https://res.cloudinary.com/zenn/image/fetch/s--eUtx65tF--/c_limit%2Cf_auto%2Cfl_progressive%2Cq_auto%2Cw_1200/https://storage.googleapis.com/zenn-user-upload/deployed-images/b5c8a0078bb649ec17ad4177.png%3Fsha%3Dce0384747cd2996173c8fa9c3c97753539f6f2a1)  
*check が success になった時の画面*

ワークフローが完了すると「ピロリン」と音が鳴ります。CI 終わらないかな〜と Pull Request の画面を見に行くのをやめられそうでいいですね。

記事の内容としては実装についても触れられていたのが良かったです。CLI から音を出す方法や Mac での通知の出し方など参考になりそうです。

*本項の執筆者: [@uta8a](https://zenn.dev/uta8a)*

# テストの Sharding を効率化する Tenbin というツールを作った

ここで言う Sharding とは、ある単位（e.g. 実行時間）でテストを均等に分割することです。分割されたテストを並列に実行して、テストにかかる時間を短縮することが目的です。紹介記事では、この Sharding を従来手法よりいい感じにできる実装と、それを Jest/Vitest/Playwright に組み込む方法が紹介されています。（「いい感じにできる実装」という部分をより詳しく話すと、Sharding から帰着できる数分割問題という計算問題の近似解を求めるアルゴリズムを実装したよ、という感じです。）

Sharding をちゃんと解こうとするのはあんまり見かけないのでいいね！！！！という気持ちです。ちゃんとした姿勢で近似解を求めるならその精度も気になるところですが、今回実装されている手法は、最適値の $\frac{4}{3}-\frac{1}{3m}$ 倍以下なのでいい感じです（$m$ は分割する個数）（c.f. [数分割問題 | opt100](https://scmopt.github.io/opt100/76npp.html#%E8%A4%87%E6%95%B0%E8%A3%85%E7%BD%AE%E3%82%B9%E3%82%B1%E3%82%B8%E3%83%A5%E3%83%BC%E3%83%AA%E3%83%B3%E3%82%B0%E5%95%8F%E9%A1%8C)）。計算量も $\mathcal{O}(n \log n)$（$n$ は分割前の個数）です。

Greedy に解くより大抵全然いい結果が得られるので、どんどん取り入られてほしい（取り入れていきたい）ですね。

*本項の執筆者: [@ajfAfg](https://zenn.dev/arjef)*

Productivity Weekly で出たネタを全て紹介したいけど紹介する体力が持たなかったネタを一言程度で書くコーナーです。

- **news 📺**
	- [Copilot Autofix can now be generated with the REST API (Public Preview) - GitHub Changelog](https://github.blog/changelog/2024-12-17-copilot-autofix-can-now-be-generated-with-the-rest-api-public-preview/)
		- Copilot Autofix が REST API 経由で生成を要求できるようになりました
		- 使い道としてはアラートができたら速攻で autofix させる…とか？
	- [Google、Web IDE上で自然言語を適切なコマンドラインに変換して実行できる「Interactive Chat」プレビュー公開。Project IDXの新機能として － Publickey](https://www.publickey1.jp/blog/24/googleweb_ideinteractive_chatproject_idx.html)
		- Google が開発している IDE、Project IDX に「Interactive Chat」機能が追加されました
		- 自然言語の指示を元にコマンドラインのコマンドを生成して実行できるようです

# あとがき

年の瀬ですね。紅白を見ながらこのあとがきを書いています。あと一個今年の分も残ってるので、それまで出してから年を越したい。

サイボウズの生産性向上チームでは社内エンジニアの開発生産性を上げるための活動を行なっています。そんな生産性向上チームが気になる方は下のリンクをクリック！

[GitHubで編集を提案](https://github.com/korosuke613/zenn-articles/blob/main/articles/productivity-weekly-20241218.md)

9
