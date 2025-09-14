---
created: 2025-05-11T16:13:42+09:00
updated: 2025-05-17T14:47:57+09:00
title: HTMLメモ
category: tech
tags: [html]
is_completed: false
priority: 9002
summary: HTMLに関する知識で気になったことをまとめます。
---

# タグについて

## Aタグ

リンクを新しいタブで表示させるときは、target属性に`_blank`を設定する。

```html
<a href="hoge" target="_blank"></a>
```

しかし、これには少し脆弱性があるらしく、よりセキュアにするためには`rel="noopener"`や`ref="noreferer"`とすれば良いらしい。

ただ、最近のブラウザでは、何も指定しなくても`target="_blank"`指定でデフォルトで、`ref="noopener"`がつくそう。

`ref="noreferer"`はアクセス解析に邪魔になったりするので、`rel="noopener"`だけをつければ良いが、デフォルトで有効になっているので、何も書かなくても良さそう。

ただちゃんと考えてつけてますという意味合いを込めて、`ref="noopener"`はつけても良さそうか？
