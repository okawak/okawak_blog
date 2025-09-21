# ADR-0002: ドメイン層の純粋化

## ステータス

Accepted

## コンテキスト

Rust-First Architectureの採用に伴い、ドメイン層からI/O操作と非同期処理を完全に排除する必要が生じた。

### 現在の状況
- ドメイン層にasync/awaitが混在
- 外部依存（tokio, async-trait）の混入
- WASM環境でのコンパイルエラー
- テスト実行時の複雑性

### 解決すべき問題
- **WASM互換性**: tokioの`net`機能がWASMで利用不可
- **純粋性**: ドメインロジックとI/O処理の分離不足
- **テスト容易性**: 非同期テストの複雑さ
- **型安全性**: 実行時エラーの可能性

### 制約や要求事項
- ドメインロジックの純粋性維持
- WASM環境での完全動作
- ユニットテストの簡素化
- ビジネスルールの明確化

## 決定

ドメイン層を**完全に純粋**にして、全てのI/O操作をサーバー層に移譲。

### 実装方法

1. **純粋関数設計**
   ```rust
   // Before: async trait with I/O
   #[async_trait]
   trait ArticleService {
       async fn create_article(&self, data: CreateData) -> Result<Article>;
   }
   
   // After: pure function
   pub fn generate_article_from_data(data: CreateData) -> Result<Article, DomainError> {
       // ビジネスロジックのみ、I/Oなし
   }
   ```

2. **依存関係の除去**
   ```toml
   # Removed
   tokio = { version = "1", features = ["rt-multi-thread"] }
   async-trait = "0.1"
   
   # Pure dependencies only
   serde = { version = "1", features = ["derive"] }
   chrono = { version = "0.4", features = ["serde"] }
   thiserror = "2"
   uuid = { version = "1", features = ["v4", "serde"] }
   ```

3. **ビジネスルール関数化**
   ```rust
   pub mod business_rules {
       // スラッグ生成（純粋関数）
       pub fn generate_slug_from_title(title: &Title) -> Result<Slug> { /* ... */ }
       
       // 記事ステータス変更（純粋関数）
       pub fn transition_article_status(article: Article, to: Status) -> Result<Article> { /* ... */ }
       
       // バリデーション（純粋関数）
       pub fn validate_article_data(data: &CreateData) -> Result<()> { /* ... */ }
   }
   ```

### 重要な設計原則

- **副作用の排除**: 全ての関数は副作用なし
- **決定論的**: 同じ入力に対して常に同じ出力
- **合成可能**: 関数の組み合わせによる複雑な処理
- **型による制約**: 不正な状態をコンパイル時に検出

## 結果

### ポジティブな影響

- **WASM完全対応**: フロントエンドでのドメインロジック利用可能
- **テスト簡素化**: 非同期ランタイム不要
- **デバッグ容易性**: 純粋関数によるステップ実行
- **並行安全性**: 副作用なしによる安全な並列処理
- **型安全性向上**: コンパイル時バリデーション強化

### ネガティブな影響

- **リファクタリング範囲**: 既存コードの大幅修正
- **データフロー変更**: I/O処理をサーバー層で管理する必要
- **学習コスト**: 関数型プログラミングパラダイムの理解

### 代替案

1. **async/awaitの継続使用**
   - トレードオフ: WASM互換性とテスト複雑性
   - リスク: 将来の拡張性制限

2. **条件付きコンパイル（#[cfg]）**
   - トレードオフ: コード複雑性とメンテナンス負荷
   - リスク: プラットフォーム固有バグ

3. **ドメイン層分割**
   - トレードオフ: アーキテクチャ複雑化
   - リスク: 責任範囲の曖昧化

## 実装詳細

### ファイル構造
```
domain/
├── src/
│   ├── entities/          # エンティティ定義
│   ├── business_rules.rs  # ビジネスルール（純粋関数）
│   ├── error.rs          # ドメインエラー
│   └── lib.rs            # モジュール構成
└── Cargo.toml            # 純粋依存関係のみ
```

### 典型的な純粋関数の例
```rust
pub fn calculate_reading_time(content: &str) -> Duration {
    let word_count = content.split_whitespace().count();
    let wpm = 200; // 平均読書速度
    Duration::from_secs((word_count * 60 / wpm) as u64)
}
```

## 参考資料

- [Functional Programming in Rust](https://doc.rust-lang.org/book/ch13-00-functional-features.html)
- [WASM and Async Rust](https://rustwasm.github.io/wasm-bindgen/reference/async.html)
- [Pure Functions in Domain Design](https://martinfowler.com/articles/domain-oriented-observability.html)
- [Leptos WASM Compatibility](https://leptos.dev/deployment/index.html)

---

**作成者**: Claude (AI Assistant)  
**作成日**: 2025-09-20  
**最終更新**: 2025-09-20