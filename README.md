<p>
  [![Deploy Obsidian to S3](https://github.com/okawak/okawak_blog/actions/workflows/deploy.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/deploy.yml)
  [![Security audit](https://github.com/okawak/okawak_blog/actions/workflows/security.yml/badge.svg)](https://github.com/okawak/okawak_blog/actions/workflows/security.yml)
</p>

# ぶくせんの探窟メモ

https://www.okawak.net

## 準備

`stylance`を使って、`import_style!();`マクロを使っているので、nightly の Rust が必要になる。

現在はプロジェクトディレクトリに対して、Nightly に切り替えるように、次のコマンドを実行する必要がある。

```shell
rustup override set nightly
```

この後に、target に wasm32-unknown-unknown を指定する

```shell
rustup target add wasm32-unknown-unknown
```
