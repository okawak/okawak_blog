# Obsidian コンテンツテンプレート

## 目的

この文書は、publisher が処理できる Obsidian Markdown の最小テンプレートをまとめたものである。source of truth は private な Obsidian リポジトリ側にあり、この public リポジトリには通常ファイルとして commit しない。

## 前提

- Markdown は LF 改行の frontmatter を前提にする
- `is_completed: true` のものだけを公開対象として扱う
- `kind` によって `article` / `category` / `page` / `home` を判定する
- category 配下のディレクトリ構造は `section_path` として path から導出する

## 推奨ディレクトリ構造

```text
Publish/
  about.md
  home.md
  tech/
    landing.md
    rust/
      async.md
      ownership.md
    web/
      leptos.md
  daily/
    landing.md
    diary/
      2026-04-01.md
```

この構造から次のように解釈する。

- `about.md`
  - `kind=page`
  - `/about`
- `home.md`
  - `kind=home`
  - `/` の intro fragment
- `tech/landing.md`
  - `kind=category`
  - `/tech`
- `tech/rust/async.md`
  - `kind=article`
  - `/tech/<slug>`
  - `section_path=["rust"]`

## 1. 通常記事

配置例:

```text
Publish/tech/rust/async.md
```

テンプレート:

```yaml
---
title: "Async Rust のメモ"
kind: article
category: tech
tags: ["rust", "async"]
summary: "記事一覧やメタ情報に出す短い説明。"
is_completed: true
priority: 1
created: "2026-04-12T10:00:00+09:00"
updated: "2026-04-12T10:00:00+09:00"
---
```

```md
# Async Rust のメモ

本文をここに書く。
Obsidian link や bookmark 埋め込みを含めてよい。
```

メモ:

- `kind` を省略した場合は `article` として扱う
- `category` は必須
- `Publish/tech/rust/async.md` のような path なら `section_path=["rust"]` が自動で付く

## 2. カテゴリトップページ

配置例:

```text
Publish/tech/landing.md
```

テンプレート:

```yaml
---
title: "技術メモ"
kind: category
category: tech
summary: "技術カテゴリの導入文。"
is_completed: true
created: "2026-04-12T10:00:00+09:00"
updated: "2026-04-12T10:00:00+09:00"
---
```

```md
# 技術メモ

このカテゴリで扱う内容の説明を書く。
カテゴリ配下の記事一覧は site 側で差し込まれる。
```

メモ:

- ファイル名は固定しない
- 同じカテゴリ配下で `kind=category` を持つ Markdown を、そのカテゴリの landing page として扱う
- 対応する artifact は `site/categories/<category>/page.html` と `site/categories/<category>/index.json`

## 3. 固定ページ

配置例:

```text
Publish/about.md
```

テンプレート:

```yaml
---
title: "About"
kind: page
page: about
summary: "このサイトについて"
is_completed: true
created: "2026-04-12T10:00:00+09:00"
updated: "2026-04-12T10:00:00+09:00"
---
```

```md
# About

自己紹介やサイトの説明を書く。
```

メモ:

- `page` は固定ページ key
- 現在の主要用途は `about`
- 対応する artifact は `site/pages/about.json`

## 4. Home fragment

配置例:

```text
Publish/home.md
```

テンプレート:

```yaml
---
title: "Home"
kind: home
summary: "トップページ導入文"
is_completed: true
created: "2026-04-12T10:00:00+09:00"
updated: "2026-04-12T10:00:00+09:00"
---
```

```md
このサイトは、Obsidian で管理した Markdown を artifact に変換して公開しています。
```

メモ:

- home 全体を Markdown に置き換えるのではなく、intro 部分の fragment として扱う
- 対応する artifact は `site/pages/home.json`

## 必須フィールドの目安

### article

- `title`
- `category`
- `is_completed`
- `created`
- `updated`

### category

- `title`
- `kind: category`
- `category`
- `is_completed`
- `created`
- `updated`

### page

- `title`
- `kind: page`
- `page`
- `is_completed`
- `created`
- `updated`

### home

- `title`
- `kind: home`
- `is_completed`
- `created`
- `updated`

## 運用上の注意

- 同じ `page` key を複数ファイルで使わない
- 同じカテゴリ配下で `kind=category` を複数作らない
- 未完成の下書きは `is_completed: false` のままにする
- category ごとの記事グルーピングは frontmatter ではなくディレクトリ構造で表現する
