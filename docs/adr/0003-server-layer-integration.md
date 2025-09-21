# ADR-0003: サーバー層統合設計

## ステータス

Accepted

## コンテキスト

Rust-First Architectureおよびドメイン層の純粋化に伴い、従来分散していたバックエンド機能を統合する必要が生じた。

### 現在の状況
- application, service, infrastructureクレートが分散
- Clean Architectureの厳密な層分離
- 複雑なtrait依存関係
- 複数バイナリによるデプロイ複雑性

### 解決すべき問題
- **デプロイ複雑性**: 複数のバイナリとサービス管理
- **パフォーマンス**: 過度な抽象化によるオーバーヘッド
- **メンテナンス**: 分散したコードベースの管理負荷
- **型安全性**: 実行時のDI（Dependency Injection）エラー

### 制約や要求事項
- 単一バイナリでのデプロイ
- Leptos Server Functionsとの統合
- AWS S3との連携
- 高パフォーマンスの維持

## 決定

バックエンド機能を**serverクレート**に統合し、単一責任の統合サーバーとして設計。

### 統合設計

1. **モジュール構成**
   ```
   server/
   ├── src/
   │   ├── usecases/          # ビジネスロジック統合
   │   ├── ports/             # 最小限のtrait定義
   │   ├── infrastructure/    # 具象実装（S3, DB等）
   │   ├── handlers/          # HTTP/Leptos統合
   │   ├── config.rs          # 設定管理
   │   ├── server.rs          # 統合アプリケーション
   │   └── main.rs            # エントリーポイント
   └── Cargo.toml
   ```

2. **責任分離**
   - **usecases**: ドメインロジックとインフラの橋渡し
   - **ports**: 必要最小限のinterface定義
   - **infrastructure**: S3, Database等の具象実装
   - **handlers**: Axum + Leptos統合

3. **統合フロー**
   ```rust
   // main.rs
   let (repository, storage) = initialize_services(s3_client, bucket).await;
   let app = create_app(repository, storage);
   axum::serve(listener, app).await?;
   ```

### 重要な設計原則

- **Composition over Inheritance**: trait継承より具象合成
- **単一責任統合**: 各モジュールは明確な単一責任
- **実行時安全性**: コンパイル時依存解決
- **パフォーマンス優先**: ゼロコスト抽象化の活用

## 結果

### ポジティブな影響

- **デプロイ簡素化**: 単一バイナリによる運用簡素化
- **パフォーマンス向上**: 直接的な関数呼び出し
- **型安全性**: コンパイル時依存解決
- **開発効率**: 統合されたコードベース
- **Leptos統合**: Server Functionsとの自然な統合

### ネガティブな影響

- **初期複雑性**: 統合設計の理解コスト
- **テスト戦略**: モック作成の制約
- **モジュール結合**: 適切な境界設計の重要性

### 代替案

1. **マイクロサービス継続**
   - トレードオフ: 運用複雑性とネットワークレイテンシー
   - リスク: サービス間通信の信頼性

2. **従来のレイヤード設計**
   - トレードオフ: 冗長性とパフォーマンス
   - リスク: WASM互換性とDI複雑性

3. **完全モノリス**
   - トレードオフ: 責任混在と拡張性制限
   - リスク: 長期メンテナンス性

## 実装詳細

### UseCases統合例
```rust
pub struct BlogUseCases<R, S> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R, S> BlogUseCases<R, S>
where
    R: ArticleRepository + 'static,
    S: FileStorage + 'static,
{
    pub async fn create_article(&self, title: String, content: String, category: Category) -> Result<Article> {
        // 1. ドメインロジック呼び出し（純粋関数）
        let article_data = domain::validate_and_create_data(title, content, category)?;
        let article = domain::generate_article_from_data(article_data)?;
        
        // 2. インフラストラクチャ操作
        self.repository.save(&article).await?;
        self.storage.put(&article.content_key(), article.content().as_bytes()).await?;
        
        Ok(article)
    }
}
```

### Handler統合例
```rust
pub async fn create_article<R, S>(
    State(state): State<Arc<AppState<R, S>>>,
    Json(req): Json<CreateArticleRequest>,
) -> Result<Json<ApiResponse<Article>>, StatusCode>
where
    R: ArticleRepository + 'static,
    S: FileStorage + 'static,
{
    match state.use_cases.create_article(req.title, req.content, req.category).await {
        Ok(article) => Ok(Json(ApiResponse::success(article))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}
```

### 統合アプリケーション
```rust
pub fn create_app(
    repository: Arc<MemoryArticleRepository>,
    storage: Arc<S3Storage>,
) -> Router {
    let use_cases = Arc::new(BlogUseCases::new(repository, storage));
    let app_state = Arc::new(AppState { use_cases });
    
    Router::new()
        .route("/health", get(health_check))
        .nest("/api", create_api_router::<MemoryArticleRepository, S3Storage>())
        // 将来: Leptos SSR統合
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}
```

## 運用面での利点

- **systemd統合**: 単一サービスファイルで管理
- **ログ集約**: 統一されたログストリーム
- **監視簡素化**: 単一エンドポイントでの監視
- **バックアップ**: 単一バイナリのバックアップ

## 参考資料

- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [Leptos Server Functions](https://leptos.dev/server/25_server_functions.html)
- [Rust Module System](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [Single Binary Deployment](https://12factor.net/processes)

---

**作成者**: Claude (AI Assistant)  
**作成日**: 2025-09-20  
**最終更新**: 2025-09-20