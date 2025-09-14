---
created: 2025-05-04T16:50
updated: 2025-05-10T23:58:11+09:00
title: thiserrorとanyhow
category: tech
tags: [Rust, anyhow, thiserror, error-handling]
is_completed: true
priority: 1
summary: Rustでエラーハンドリングする際のベストプラクティスと言われているthiserrorとanyhowについてまとめた記事です。
---

# はじめに

Rustでは2種類の回復可能なエラーと回復不可能なエラーがあります。回復可能なエラーはResult型でエラー処理を行います。一方で回復不可能なエラーはpanicと呼ばれ、即座にプロセスが終了します。一般的にエラーが予期できる場合はpanicを起こさないようにして、うまくエラーハンドリングすることが求められます。

Rustでエラーハンドリングするときに、Result型をうまく使って標準ライブラリだけで処理することもできますが、より丁寧に扱うには`anyhow`や`thiserror`を使うのが便利です。

その他のクレートとして`snafu`、`eyre`, `miette`も使われているそうですが、これはまたの機会に調べてみようと思います。

結論から書くと一番良い扱い方は、`anyhow`と`thiserror`を組み合わせて、アプリケーションでは`anyhow`、ライブラリでは`thiserror`を使うのが一番良いそうです。

[thiserrorのREADME.md](https://github.com/dtolnay/thiserror)より

> Use thiserror if you care about designing your own dedicated error type(s) so that the caller receives exactly the information that you choose in the event of failure. This most often applies to library-like code. Use [Anyhow](https://github.com/dtolnay/anyhow) if you don't care what error type your functions return, you just want it to be easy. This is common in application-like code.

[anyhowのREADME.md](https://github.com/dtolnay/anyhow)より

> Use Anyhow if you don't care what error type your functions return, you just want it to be easy. This is common in application code. Use [thiserror](https://github.com/dtolnay/thiserror) if you are a library that wants to design your own dedicated error type(s) so that on failures the caller gets exactly the information that you choose.

# Anyhow

## 特徴

anyhowは`Box<dyn std::error::Error>`型(トレイトオブジェクト)のように動的にエラーを内包するような型で、この標準ライブラリのトレイトオブジェクトと同じように使うことができます。しかし、大きく違う点としてこの3点が挙げられます。

- エラー型として実装されるトレイト境界が、`Send + Sync + 'static` であることが要求される(`Box<dyn Error + Send + Sync + static>`である)
- **バックトレース**を利用できる
- 標準ライブラリのトレイトオブジェクトはfat pointerで2ワードを必要とするが、anyhowは**narrow pointerで1ワードである**

異なるエラーを一元的に扱える、かつ後半二つの特徴により、効率的かつエラーの詳細情報も得ることができるといった優れたライブラリとなっています。

また、大きな特徴としてRust開発者として有名なDavid Tolnay氏(dtolnay氏)によって開発、管理されているので、安定したライブラリでエラー処理のデファクトスタンダードとなっています。

## Cargo.toml

まず`anyhow`を使うためにはクレートを追加します。

```bash
cargo add anyhow
# 執筆時点での最新バージョンは1.0.98
```

featureとして、no_stdを指定することもできますが、組み込み系など限られた場合でない限り、featureを明示的に何か追加で書く必要はありません。

<div class="bookmark">
  <a href="https://lib.rs/crates/anyhow/features">Feature flags of Anyhow crate</a>
</div>

Rustのバージョンがv1.65よりも古い場合は、バックトレースを得るためにfeaturesとしてbacktraceを指定する必要があったようですが、それ以降のバージョンでは標準ライブラリにstd::backtraceが実装され、デフォルトでstd featureが有効になるため、特に指定する必要はなさそうです。

ただし、バックトレース機能を使うためには環境変数として、`RUST_BACKTRACE=1`をつける必要があります。

Cargo.tomlにはこのように記述します。

```toml
[dependencies]
anyhow = "1.0"
```

## 基本的な使い方

通常の使い方としては、Result型のエラー部分にanyhow::Errorを指定します。つまり関数のシグネチャとしては、

```rust
use anyhow::Result;

fn hoge() -> Result<()> { // Result<(), anyhow::Error>と同義
    Ok(())
}
```

といった形が基本となります。以下様々な使い方を列挙しますが、`?`演算子で自動的にanyhow::Errorオブジェクトに変換されるので、基本的には?を使うことが多いと思います。

### anyhow!マクロ

anyhow!マクロを使用することで、簡単にanyhow::Errorオブジェクトを作成することができます。具体的には、以下のような使い方ができます。

- 既存のエラー型をanyhowに変換
- エラーメッセージから生成(format文で書ける)

```rust
use anyhow::{Result, anyhow};

fn error_test(x: i32) -> Result<()> {
    // ファイルを読み取る
    std::fs::File::open("hoge.txt").map_err(|e| anyhow!(e))?;

    // format文でエラーメッセージを作る
    if x < 0 {
        return Err(anyhow!("値xが負でした: {x}"));
    }
    Ok(())
}

fn main() -> Result<()> {
    error_test(-1)?;
    // ファイルがなかった場合のエラーメッセージ
    // Error: No such file or directory (os error 2)
    // 
    // 引数が負だった場合のエラーメッセージ
    // Error: 値xが負でした: -1
    Ok(())
}
```

> 後述しますが、一番目の例で`map_err`を使って明示的にいanyhow::Errorに変換していますが、`open("hoge.txt")`の後に?を使って自動的に変換することができます。あえて長く書く必要はないので、?で簡潔に書くのが一般的だと思います。

### bail!マクロ

このマクロは早期リターンをするために用いられます。つまり、`return Err(anyhow!(_))`と同じです。なので先ほどの例のうち、returnしていた部分は

```rust
use anyhow::{Result, bail};

fn error_test(x: i32) -> Result<()> {
    // ...
    if x < 0 {
        bail!("値xが負数でした: {x}");
    }
    // xが正だった場合、何らかの正常処理が書ける
    Ok(())
}

fn main() -> Result<()> {
    error_test(-1)?;
    Ok(())
}
```

というふうに書くことができます。

### ensure!マクロ

さらにこの処理を簡潔に書くマクロがensureで、`ensure!(条件, エラーメッセージ)`のように書き、**条件がfalse** であれば、bail!のようにErrを返して関数を終了させます。逆に言うと、**条件がtrueの場合に次の処理に進みます。**

つまり上の例をさらに簡潔に書くことができます。

```rust
use anyhow::{Result, ensure};

fn error_test(x: i32) -> Result<()> {
    // ...
    ensure!(x >= 0, "値xが負数でした: {x}");
    // ...
    Ok(())
}

fn main() -> Result<()> {
    error_test(-1)?;
    Ok(())
}
```

### `?`演算子との組み合わせ

改めて`?`演算子はResultやOptionに対して用いられ、Ok(v)の場合はv(Some(v)の場合はv)を取り出し、Err(None)の場合はそれを早期リターンします。

エラー型が異なる場合、つまり`Result<T, E>`から`Result<T, F>`へ変換するときは、一般的にはEからFへFromを実装または`Into<F>`を実装する必要があります。しかし、anyhow::Errorはデフォルトで多くのエラー型に対してFromを実装しており、特に`E: std::error::Error + Send + Sync + 'static`の場合は、自動的にanyhow::Errorに変換されます。一般的なクレートのエラー型はこのEのトレイト実装を持っているので、?演算子で自動的にanyhow::Errorに変換させることができます。

つまり最初の例は次のように見通しよく書くことができるということですね！

```rust
use anyhow::{Result, ensure};

fn error_test(x: i32) -> Result<()> {
    std::fs::File::open("hoge.txt")?;
    ensure!(x >= 0, "値xが負数でした: {x}");
    // ...
    Ok(())
}

fn main() -> Result<()> {
    error_test(-1)?;
    Ok(())
}
```

多くのエラー型を一度に扱う場合はmap_errなどを書くと冗長になるので、このように簡潔に書けるのが魅力です。

> 扱っているエラー型がstd::error::Errorを実装していない場合は、?で自動的に変換されません。その場合はmap_errやanyhow!を使って明示的に変換することが必要となります。

## エラーに追加情報をつける

anyhowには`Context`というトレイトがあり、エラーメッセージに情報を付け加えることができます。このトレイト経由で`.context()`メソッドと`.with_context()`が使えるようになります。

> トレイトをimportすることが大事で、名前は問題ではないので、`use anyhow::Context as _;`とすると名前が被ることなくcontextメソッドを使うことができます。

この二つの特徴は、

- `context`: 引数で得られる文字列を使って、anyhow::Errorを作り、コンテキストを追加(std::error::Errorのsourceに情報を追加)することができる。**先行評価(eagerly evaluated)** されるので、エラーが発生しなくても文字列の生成は行われる。固定値の場合は良いが、format!で整形したり関数を使ったりすると、無駄な処理が発生する。
- `with_context`: 引数にはクロージャを入れる。つまり**遅延評価(lazily evaluated)** されるので、エラーが発生した時のみ評価される。

であり、使い分けとしては、"error message"のような静的なメッセージの場合はcontextで十分ですが、動的に(リッチな感じのメッセージ)メッセージを作りたい場合は基本的にはwith_contextを使う方が良いかと思います。

```rust
use anyhow::Context as _;
use anyhow::{Result, ensure};

fn error_test(x: i32) -> Result<()> {
    let filename = "hoge.txt";
    std::fs::File::open(filename)
        .with_context(|| format!("コンテキスト: failed to open {filename}"))?;
    // ...
    ensure!(x >= 0, "値xが負数でした: {x}");
    Ok(())
}

fn main() -> Result<()> {
    error_test(-1)?;
    // ファイルがなかった時のエラーメッセージは次のようになります。
    // Error: コンテキスト: failed to open hoge.txt
    //
    // Caused by:
    // No such file or directory (os error 2)
    Ok(())
}
```

### Optionに対するcontext

今まではResult型に対する処理を述べてきましたが、ContextはOption型にも実装されており、OptionのNoneに対してもanyhowのエラーに変換することができます。

## バックトレース

最後にanyhowのメイン機能(?)とも言えるバックトレースについて紹介します。実行するときに`RUST_BACKTRACE=1 cargo run`のように環境変数を設定すると情報を得ることができます。

まずはバックトレースなしで、次のようなコードを実行してみます。

```rust
use anyhow::Context as _;
use anyhow::{Result, anyhow};

fn parent() -> Result<()> {
    child1().context("外側の関数のコンテキスト")?;
    Ok(())
}

fn child1() -> Result<()> {
    child2().context("中間の関数のコンテキスト")?;
    Ok(())
}

fn child2() -> Result<()> {
    Err(anyhow!("内側の関数のコンテキスト、ここでエラーが発生"))
}

fn main() -> Result<()> {
    parent()
    // 出力はこうなる
    // Error: 外側の関数のコンテキスト
    //
    // Caused by:
    //     0: 中間の関数のコンテキスト
    //     1: 内側の関数のコンテキスト、ここでエラーが発生
}
```

このように関数同士が入り組んでいる場合、どこでエラーが起きたか分かりづらくなっていますが、コンテキストを適切につけていれば、実はバックトレースをオンにしなくても、何となくどこに原因があるか分かりやすくすることができます。(一番下の部分が実際の原因となります。)

コンテキストをつけなかった場合、単純に`cargo run`するとこんな風になります。

```rust
use anyhow::{Result, anyhow};

fn parent() -> Result<()> {
    child1()?;
    Ok(())
}

fn child1() -> Result<()> {
    child2()?;
    Ok(())
}

fn child2() -> Result<()> {
    Err(anyhow!("エラー"))
}

fn main() -> Result<()> {
    parent()
    // 出力はこうなる
    // Error: エラー
}
```

複雑な構造を持っていて、コンテキストをつけていなかった場合、どこでエラーが起きたか分かりにくくなってしまいます。

そこで、バックトレースを有効にして実行するとこのような出力となります。小さくてすみません。環境によってPathは変わると思います。

![image](https://d1fhrovvkiovx5.cloudfront.net/62846baef7798234576089d9a00303ff.png)

上から順に辿っていけば、まず初めに`error_test::child2`という部分が目に入るので、ここが原因だなと突き止めることができるかと思います。

# Thiserror

次にanyhowと相補的な関係にあるthiserrorについて説明します。

## 特徴

anyhowのデメリットとして、全てのエラーがanyhow::Error型に変換されてしまい、エラー型の情報が失われてしまうということがあります。エラー型の情報を残したまま、独自のエラー型を定義しやすくするためのクレートがthiserrorです。

また、このクレートもDavid Tolnay氏(dtolnay氏)によって開発、管理されている、エラー管理のデファクトスタンダートとなっているクレートです。

### 1. 一般的な独自エラーの定義方法

thiserrorの前に独自のエラー型を定義する時の一般的な書き方を整理しておきます。

```rust
enum MyError {
    IoError(std::io::Error),
    ParseError(std::num::ParseIntError),
}

fn read_and_parse_file(path: &str) -> Result<i32, MyError> {
    let content = std::fs::read_to_string(path).map_err(MyError::IoError)?;
    let number: i32 = content.trim().parse().map_err(MyError::ParseError)?;
    Ok(number)
}
```

この`read_and_parse_file`関数は通常だと二つのエラー型、std::io::Errorとstd::num::ParseIntErrorを返す可能性があります。これを同時に扱うためには、二つの異なるエラーを一つの型で表せるような型を定義する必要があり、それが`MyError`です。

`map_err`を使って明示的にエラー型をMyErrorに変換しています。このように型情報を残しておくと、例えばmain関数の中で、次のように分岐処理を行うことができます。

```rust
fn main() {
    let path = "number.txt";
    let num = match read_and_parse_file(path) {
        Ok(n) => n,
        Err(MyError::IoError(e)) => {
            eprintln!("ファイルの読み込みエラー: {e}");
            return;
        }
        Err(MyError::ParseError(e)) => {
            eprintln!("数値の解析エラー: {e}");
            return;
        }
    };
    println!("読み込んだ数値: {num}");
}
```

read_and_parse_fileでmap_errを使っていましたが、エラーが増えていくとこれを毎回書くのは面倒です。anyhowのように?で自動的に変換してくれたら嬉しいですよね。

### `?`で変換してくれるようにする

std::convert::Fromトレイトを実装することで、?演算子を使って自動でエラー型をMyErrorに変換させることができます。Fromトレイトの実装はこのような形です。

```rust
enum MyError {
    IoError(std::io::Error),
    ParseError(std::num::ParseIntError),
}

impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> Self {
        MyError::IoError(err)
    }
}

impl From<std::num::ParseIntError> for MyError {
    fn from(err: std::num::ParseIntError) -> Self {
        MyError::ParseError(err)
    }
}
```

イメージとしては「for MyError」の部分でMyErrorに対して、「`From<E>`」で`From<E>`を実装します、という形で、fromの引数のエラーをSelf(MyError)の中のfieldに変換します、という意味合いになります。

これで関数をスッキリかけます。

```rust
fn read_and_parse_file(path: &str) -> Result<i32, MyError> {
    let content = std::fs::read_to_string(path)?;
    let number: i32 = content.trim().parse()?;
    Ok(number)
}
```

しかし、エラー型が増えるごとにいちいちFromの実装を書くのも面倒です。thiserrorはその部分を簡単に尚且つ効果的に行えるようにした便利なクレートです。

## Cargo.toml

thiserrorもCargoでクレートを追加します。

```bash
cargo add thiserror
# 執筆時点での最新バージョンは2.0.12
```

こちらも特にfeaturesとして指定する必要はありません。

<div class="bookmark">
  <a href="https://lib.rs/crates/thiserror/features">Feature flags of Thiserror crate</a>
</div>

## Derive属性をつける

まず、独自のエラー型に`thiserror::Error`トレイトをderive属性につけることで、このthiserror::Errorトレイトの基本実装を自動で行ってくれます。

```rust
// まだエラーが出ます
use thiserror::Error;

#[derive(Error)]
enum MyError {
    IoError(std::io::Error),
    ParseError(std::num::ParseIntError),
}
```

これにより、

- std::error::Errorトレイトが実装 → 例えば、エラーの原因を辿るsourceメソッドなどが実装される
- std::fmt::Displayトレイトが実装 → 後述する`#[error(”hoge”)]`属性をつけることで、エラーメッセージを表示するfmtメソッドが実装される
- std::convert::Fromトレイトが実装 → 後述する`#[from]`属性をつけることで`From<E> for MyError`が自動生成される

これらの機能が付け加わります。また、std::error::ErrorトレイトはDisplayとDebugトレイトの実装も要求するので、自分で実装するか、`#[derive(Error, Debug)]`とすることが多いです。

一旦これらの機能をフルで使った場合の例はこちらになります。

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("ファイルの読み込みエラー: {0}")]
    IoError(#[from] std::io::Error),

    #[error("数値のパースエラー: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}
```

## Error属性

エラーメッセージを定義するための属性で、ここに書いたものがDisplay実装に用いられることになります。ちなみにDisplay実装はこのような形です。[公式ドキュメント](https://doc.rust-lang.org/std/fmt/trait.Display.html)より。

```rust
use std::fmt;

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

let origin = Point { x: 0, y: 0 };

assert_eq!(format!("The origin is: {origin}"), "The origin is: (0, 0)");
```

error属性に単純な静的な文字列#[error("hoge")]を指定することもできますが、動的にメッセージを作ることができます。

Display実装との対応だと次のようなものが作れます。[ドキュメント](https://docs.rs/thiserror/latest/thiserror/)より。

- `#[error(”{var}”)]` ↔ `write!(”{}”, self.var)` (Display出力)
- `#[error(”{0}”)]` ↔ `write!(”{}”, self.0)` (Display出力)
- `#[error(”{var:?}”)]` ↔ `write!(”{:?}”, self.var)` (Debug出力)
- `#[error(”{0:?}”)]` ↔ `write!(”{:?}”, self.0)` (Debug出力)

この0はエラー型の引数を表しており、例えば複数の引数を持つエラー型を定義した場合は、0, 1という風に書けば、複数表示させることが可能です。

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("一つ目の引数: {0}, 二つ目の引数: {1}")]
    TwoMessageError(String, String),
}

fn error_test() -> Result<(), MyError> {
    Err(MyError::TwoMessageError(
        "message 1".to_string(),
        "message 2".to_string(),
    ))
}

fn main() {
    if let Err(e) = error_test() {
        eprintln!("{e}");
        // 出力は以下のようになる
        // 一つ目の引数: message 1, 二つ目の引数: message 2
    }
}
```

> 注意点として、`main() -> Result<(), MyError>` として?演算子でエラー処理した場合はDisplayではなくDebug表示となるので、error属性で設定したDisplay表示はされません。(これは、main関数はstd::process::Terminationトレイトを実装している必要があり、Debug出力するような実装が行われているためです。)Display出力させるためには、明示的に{e}として表示させる必要があります。ここで、anyhowは独自でTerminationトレイトを実装しており、そこではDebugではなくDisplay表示させているので、`main() -> anyhow::Result<()>`とすると、?演算子でも設定したDisplay表示が見えます。

エラーに構造体を使う場合の出力などもあるので、こちらは公式ドキュメントを参照してください。また**thiserrorのv2.0.0以降では、エラーのフォーマット処理を他の関数で定義する**ことも可能になりました。cf. [githubリリースより](https://github.com/dtolnay/thiserror/releases/tag/2.0.0)

`#[error(fmt = fmt関数名)]`という形式で属性を書き、fmtの関数を別で定義できます。Display実装の中のfmt関数を自由に書けるというイメージですね。

### `#[error(transparent)]`

error属性の特殊な例として、transparentを指定できます。これはエラーメッセージを追加せず、そのまま外側のエラーとして扱うことができます。例えば以下の例だと、

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("数値のパースエラー: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}

// read_and_parse_file関数は同様

fn main() {
    let path = "number.txt";
    let num = match read_and_parse_file(path) {
        Ok(n) => n,
        Err(MyError::IoError(e)) => {
            eprintln!("{e}");
            // No such file or directory (os error 2)
            return;
        }
        Err(MyError::ParseError(e)) => {
            eprintln!("数値の解析エラー: {e}");
            return;
        }
    };
    println!("読み込んだ数値: {num}");
}
```

std::io::Errorのメッセージである「No such file or directory (os error 2)」というメッセージがそのままMyError::IoErrorのメッセージとして出力されます。

このtransparentの主な用途としては大きく二つあるようです。

- その他のエラーケースとして包括的に受け取る。

```rust
#[derive(Error, Debug)]
pub enum MyError {
    // ...
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

- 内部的なエラー処理を隠蔽し、外部には単一のエラー型のみを公開する(内部構造が変わっても外部からの変更は必要ない。[公式ドキュメントより](https://docs.rs/thiserror/latest/thiserror/))

```rust
// PublicError is public, but opaque and easy to keep compatible.
#[derive(Error, Debug)]
#[error(transparent)]
pub struct PublicError(#[from] ErrorRepr);

impl PublicError {
    // Accessors for anything we do want to expose publicly.
}

// Private and free to change across minor version of the crate.
#[derive(Error, Debug)]
enum ErrorRepr {
    ...
}
```

## Source, Backtraceなどの属性

他にも属性について色々書かれていましたが、おそらく、backtrace機能はnightlyのコンパイラを使う必要がありそうです。

<div class="bookmark">
  <a href="https://github.com/dtolnay/thiserror/issues/257">https://github.com/dtolnay/thiserror/issues/257</a>
</div>

つまりこれは現状だと通常のコンパイラではコンパイルできません。

```rust
#[derive(Error, Debug)]
enum MyError {
    #[error("I/Oエラー: {source}")]
    Io {
        #[from]
        source: std::io::Error,
        backtrace: Backtrace,
    },
}
```

つまりthiserrorでバックトレースを得るのは現状難しいです。

## ネストされたエラー

anyhowで行った例のように、関数同士が複雑に組み合わさった場合のエラーメッセージの例を見てみます。thiserrorだけを使った場合このようになります。

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("数値のパースエラー: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}

fn parent() -> Result<(), MyError> {
    child1()?;
    Ok(())
}

fn child1() -> Result<(), MyError> {
    child2()?;
    Ok(())
}

fn child2() -> Result<(), MyError> {
    "abc".parse::<i32>()?;
    Ok(())
}

fn main() -> Result<(), MyError> {
    parent()
    // 出力はこうなる
    // Error: ParseError(ParseIntError { kind: InvalidDigit })
}
```

`RUST_BACKTRACE=1`として実行しても、バックトレースは得られず、エラーメッセージは同じものが表示されます。では、一般的にどのようにエラーハンドリングを行うのがベストプラクティスと言われているかというと、anyhowとthiserrorをうまく組み合わせて使うことです。

---

以下参考です。おそらく今後backtraceをthiserrorでも得ることができるようになるかと思うので、それをちょっとみておきます。現状のコンパイラ(nightlyを使用)でbacktraceを得るにはこうします。

```rust
#![feature(error_generic_member_access)] // これが必要

use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("数値のパースエラー: {source}")]
    ParseError {
        #[from]
        source: std::num::ParseIntError,
        backtrace: std::backtrace::Backtrace,
    },
}

fn parent() -> Result<(), MyError> {
    child1()?;
    Ok(())
}

fn child1() -> Result<(), MyError> {
    child2()?;
    Ok(())
}

fn child2() -> Result<(), MyError> {
    "abc".parse::<i32>()?;
    Ok(())
}

fn main() -> Result<(), MyError> {
    parent()
}
```

Debug出力となるのでRUST_BACKTRACE=1を設定した時の出力はかなりみにくくなっていますが、一応バックトレースは得ることができます。将来的にthiserrorに機能が付け加わったらanyhowと組み合わせず、単純にthiserrorだけで、エラー処理をうまく行えることができるようになるかも…??

![image](https://d1fhrovvkiovx5.cloudfront.net/c157b981259d066019723cebb1cbd0c9.png)

また、mainの返り値だけanyhowにしてDisplay出力で見ると、全てをanyhowにしたときのバックトレースと同じようなみやすい出力が見られます。

# Anyhowとthiserrorを組み合わせた例

thiserrorはエラー型を処理できるという利点がありますが、現状バックとレースを得ることは難しそうです。(認識間違いがある可能性あります。)

そこで、main関数ではanyhowのエラーを使うことによって両方のいいとこどりをすることができます。

```rust
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("数値のパースエラー: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}

fn parent() -> Result<(), MyError> {
    child1()?;
    Ok(())
}

fn child1() -> Result<(), MyError> {
    child2()?;
    Ok(())
}

fn child2() -> Result<(), MyError> {
    "abc".parse::<i32>()?;
    Ok(())
}

fn main() -> Result<()> {
    Ok(parent()?)
    // 出力はこうなる
    // Error: 数値のパースエラー: invalid digit found in string
    //
    // Caused by:
    //     invalid digit found in string
}
```

`RUST_BACKTRACE=1`を指定した場合はバックトレースを得ることができます。注意点としては、anyhow::Errorが生成されたときにしかスタックフレームが記録されないので、main関数でエラーが起こったことしか情報は得られません。

![image](https://d1fhrovvkiovx5.cloudfront.net/d599c5dd3efdb18e716c98112449ce52.png)

# まとめ

anyhowクレートとthiserrorクレートについて詳細をまとめてみました。anyhowクレートはエラーをラップするということで、非常に使いやすい印象です。thiserrorは自分でエラー型を実装するときに定型文を省略できるという点で使いやすいクレートである印象である一方、(特殊なことをしない限り)バックトレースが得られないという点は、今後どうなっていくか期待したいところかと感じました。(Rustの開発経験がないので、backtraceはそんなに使わないとか実際の使用感は分かりませんが…)

改めてまとめると、

- anyhowは全てのエラーをひとまとめに扱う → アプリケーション層のエラー管理向け + 素早く実装したい時はこれに全部ラップする
- thiserrorは型情報も保持 → ライブラリ層の細かいエラー管理向け

と言ったところでしょうか。個人的に、main関数など、実行する関数にanyhowを使って、関数、メソッドなどライブラリ部分にはthiserrorを使う方が良いといった理解ですが、感覚が合っているかは分かりません。

> ちなみに、std::error::Errorはcore::error::Errorのre-exportになっています。core::error::ErrorはRust v1.81で安定化されました。
