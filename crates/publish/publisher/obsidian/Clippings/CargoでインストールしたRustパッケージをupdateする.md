---
title: CargoでインストールしたRustパッケージをupdateする
source: https://blog.htkyama.org/cargo_update
author:
  - "[[htkyama]]"
published: 2020-09-04
created: 2025-05-10
description: cargo-update というパッケージを cargo でインストールすることで、 cargo install でインストールしたパッケージをまとめてアップデートできる。 cargo-update cargo install-update -a…
tags: [clippings]
updated: 2025-06-01T10:29:15+09:00
---

[Skip to content](https://blog.htkyama.org/#skip-nav)

—,—1 min read

`cargo-update` というパッケージを `cargo` でインストールすることで、`cargo install` でインストールしたパッケージをまとめてアップデートできる。

- [cargo-update](https://crates.io/crates/cargo-update)

```shell
$ cargo install cargo-update
```

`cargo install-update -a` で一括でアップデートがあるか確認できる。

```shell
$ cargo install-update -a
    Updating registry 'https://github.com/rust-lang/crates.io-index'

Package       Installed  Latest   Needs update
bat           v0.15.4    v0.15.4  No
cargo-update  v4.1.1     v4.1.1   No
exa           v0.9.0     v0.9.0   No
starship      v0.44.0    v0.44.0  No

No packages need updating.
Overall updated 0 packages.
```
