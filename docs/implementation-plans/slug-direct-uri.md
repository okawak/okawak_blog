# Slug直接URI設計実装計画

## 概要
AWS S3に格納されたHTMLファイルのslugをそのままURIとして使用する、シンプルで効率的な記事表示機能を実装する。
現在のobsidian_uploaderが生成する12文字のSHA-256ハッシュベースslugを活用し、1レベルルーティングで記事にアクセスできるようにする。

## 設計方針

### URL構造（目標）
```
https://www.okawak.net/
├── a1b2c3d4e5f6     # tech記事のslug（12文字のハッシュ）
├── f6e5d4c3b2a1     # blog記事のslug
└── 9876543210ab     # daily記事のslug
```

### S3構造との対応
```
// S3構造（現在）
s3://bucket/
├── tech/a1b2c3d4e5f6.html
├── blog/f6e5d4c3b2a1.html
└── daily/9876543210ab.html

// URI構造（実装後）
https://www.okawak.net/a1b2c3d4e5f6  # → tech/a1b2c3d4e5f6.html
https://www.okawak.net/f6e5d4c3b2a1  # → blog/f6e5d4c3b2a1.html
https://www.okawak.net/9876543210ab  # → daily/9876543210ab.html
```

## アーキテクチャ設計

### Rust-First Architecture遵守
- **依存方向**: domain ← server ← web
- **ドメイン純粋性**: I/O操作なし、同期のみ、WASM対応
- **サーバー層統合**: 非同期処理、外部リソースアクセス
- **SSR中心**: Web層はServer-Side Renderingを基本とする

### 各層の責任

#### `domain`クレート（純粋ドメインロジック）
- **責任**: S3パス解析、パス生成ロジック
- **制約**: I/O操作なし、同期のみ、WASM対応
- **実装方針**:
  - S3パスからslugとカテゴリを抽出する純粋関数
  - HTMLとS3パスからArticleエンティティを生成するファクトリーメソッド
  - 既存のSlug, Category値オブジェクトを活用

#### `server`クレート（統合バックエンド）
- **責任**: S3検索、slug→記事取得、SSR統合
- **制約**: 非同期処理、S3アクセス
- **実装方針**:
  - slugから全カテゴリを検索してS3オブジェクトを特定
  - 記事一覧取得（全カテゴリ横断）
  - Leptos Server Functions でSSRに統合

#### `web`クレート（Leptosフロントエンド）
- **責任**: SSRルーティング、記事表示、SEO対応
- **制約**: SSR中心、thaw-ui使用
- **実装方針**:
  - 単一レベルパラメータルーティング
  - Server-side rendering での記事表示
  - カテゴリ別分類表示（ホームページ）

## 実装方針詳細

### Step 1: ドメイン層実装方針

#### 1.1 S3パス解析機能
**対象**: `crates/domain/src/business_rules.rs`
**方針**: 
- S3パス文字列からslugとカテゴリを抽出する純粋関数を実装
- 文字列操作のみで、パス構造 "category/slug.html" を想定
- エラーケース（無効なパス形式）への対応

#### 1.2 Article構造体拡張
**対象**: `crates/domain/src/entities.rs`
**方針**:
- HTMLコンテンツとS3パスからArticleエンティティを生成するファクトリーメソッド
- 既存のSlug, Category値オブジェクトを活用
- HTMLメタデータ抽出ロジックと統合

### Step 2: サーバー層実装方針

#### 2.1 S3Storage拡張
**対象**: `crates/server/src/infrastructure/s3_storage.rs`
**方針**:
- slugから全カテゴリを横断検索してS3オブジェクトを特定
- AWS S3 head_objectでファイル存在確認後、コンテンツ取得
- 全記事slug一覧取得機能（キャッシング用）

#### 2.2 UseCase層統合
**対象**: `crates/server/src/usecases/article_usecases.rs`  
**方針**:
- slugをキーとした記事取得ロジック
- ドメイン層の純粋関数を活用したArticle生成
- エラーハンドリング（NotFound, Storage, Domain）

#### 2.3 Server Functions
**対象**: `crates/server/src/handlers/article_handlers.rs`
**方針**:
- Leptos Server FunctionsでSSRに統合
- 記事取得、記事一覧取得のAPI提供
- エラーレスポンスの適切な処理

### Step 3: Web層実装方針（SSR中心）

#### 3.1 ルーティング設計
**対象**: `crates/web/src/app.rs`
**方針**:
- 単一パラメータでslugを受け取るシンプルルーティング
- 静的ページ（ホーム、About）と動的ページ（記事）の分離

#### 3.2 記事詳細ページ
**対象**: `crates/web/src/routes/article.rs`
**方針**:
- SSRでの記事コンテンツ表示
- Server Function経由でのデータ取得
- SEO対応（meta tags, structured data）

#### 3.3 ホームページ設計
**対象**: `crates/web/src/routes/home.rs`
**方針**:
- 全記事取得後、カテゴリ別分類表示
- SSRでの初期表示、必要に応じてクライアント側でのインタラクション
- thaw-uiコンポーネントの活用

## 実装順序とマイルストーン

### Step 1: ドメイン層（TDD）
**目標**: S3パス解析と純粋関数の実装
- [ ] S3パス解析関数のテスト作成（Red Phase）
- [ ] S3パス解析関数の実装（Green Phase）
- [ ] Article構造体拡張のテスト作成
- [ ] Article構造体拡張の実装

### Step 2: サーバー層（統合）
**目標**: S3統合とusecase実装
- [ ] S3Storage拡張のテスト作成
- [ ] S3Storage拡張の実装
- [ ] ArticleUseCasesのテスト作成
- [ ] ArticleUseCasesの実装
- [ ] Server Functions の実装

### Step 3: Web層（SSR）
**目標**: SSRルーティングと記事表示
- [ ] 単一レベルルーティングの設定
- [ ] ArticlePage コンポーネントの実装（SSR）
- [ ] ホームページのカテゴリ分類表示実装

### Step 4: 統合・最適化
**目標**: 品質保証と性能向上
- [ ] E2Eテストの実装
- [ ] エラーハンドリング改善
- [ ] パフォーマンス測定・改善

## 技術的考慮事項

### パフォーマンス対策
- **S3検索効率化**: slugから直接的なファイル検索
- **キャッシング戦略**: Server-sideでのslug→記事マッピングキャッシュ
- **SSR最適化**: 初期表示の高速化

### セキュリティ対策
- **入力検証**: slug形式の厳密な検証
- **エラー情報制限**: 適切なエラーレスポンス

### SEO対応
- **適切なURL構造**: 短く覚えやすいslug
- **メタデータ設定**: SSRでの適切なmeta tags
- **構造化データ**: 記事の構造化マークアップ

## アーキテクチャ原則の遵守

### ドメイン層の純粋性
- ✅ すべてのS3パス解析は純粋関数
- ✅ I/O操作なし、同期のみ
- ✅ WASM互換性確保

### サーバー層の責任集中
- ✅ S3アクセスはinfrastructure層
- ✅ ビジネスロジックはusecases層
- ✅ Server FunctionsでWeb層に統合

### 依存関係の方向
- ✅ domain ← server ← web
- ✅ 上位層が下位層に依存
- ✅ 逆依存なし

## 成功指標

- [ ] 12文字slugでの記事アクセス成功率 > 99%
- [ ] 記事表示レスポンス時間 < 500ms
- [ ] 全てのユニットテスト・統合テスト通過
- [ ] カテゴリ分類表示の正確性
- [ ] SEO対応（適切なmeta tags）
- [ ] SSRによる初期表示の高速化