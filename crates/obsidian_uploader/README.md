# Obsidian Uploader

Obsidian で作成される Markdown ファイルをフロントマターをそのまま残して Markdown 部分を HTML に変換します。
AWS S3 へのアップロードは Github Actions を使用することを想定しています。
S3 bucket の設定を行い、Github Secrets に

- AWS_REGION: 地域名
- AWS_ACCOUNT_ID: アカウント ID
- AWS_ROLE_NAME: ロール名
- S3_BUCKET: S3 バケット名

を入力することを想定しています。

## bookmark

通常の Markdown からの変換とともに、通常のリンク(`[hoge](example.com)`)だけではなく、リンク先情報を受け取って、ブックマークのような要素を作成します。

```html
<div class="bookmark">
  <a href="{url}">hoge</a>
</div>
```

というリンクを Markdown に書いておくと、サイトのタイトル、詳細、ドメイン、サムネイル画像を受け取って以下のように変換します。

```html
<div class="bookmark">
  <a class="bookmark-link" href="{url}" target="_blank" rel="noopener">
    <div class="bookmark-content">
      <div class="bookmark-title">{title}</div>
      <div class="bookmark-description">{desc}</div>
      <div class="bookmark-domain">{site}</div>
    </div>
    <div class="bookmark-thumb" style="background-image:url('{img}')"></div>
  </a>
</div>
```

スタイルクラス名は固定で、後はよしなにして下さい。

## 作成するファイル名

Obsidian でタイトルを日本語にすると、Markdown のファイル名も日本語になるが、扱いづらいので、適当な文字列でファイル名を作成する。
