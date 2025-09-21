# ADR-0001: Rust-First アーキテクチャの採用

## ステータス

Accepted

## コンテキスト

okawak_blogプロジェクトにおいて、従来のClean Architecture パターンではなく、Rustの特性を活かしたアーキテクチャ設計が必要となった。

### 現在の状況
- Leptosフレームワークを使用したSSRブログシステム
- マルチクレート構成によるモジュール分割
- AWS S3を使用したファイルストレージ

### 解決すべき問題
- Clean Architectureの冗長性（過度な抽象化）
- Rustの所有権システムとの不整合
- WASM環境でのコンパイル制約
- パフォーマンス重視の要求

### 制約や要求事項
- Leptos 0.8との互換性
- 単一バイナリでのデプロイ
- 型安全性の最大化
- ゼロコスト抽象化の活用

## 決定

Clean Architectureから**Rust-First Architecture**への移行を決定。

### 採用する解決策

1. **純粋ドメイン設計**
   - I/O操作を排除した同期のみのドメイン層
   - 純粋関数によるビジネスロジック実装
   - Rustの型システムを活用した制約表現

2. **サーバー層統合**
   - usecases, ports, infrastructure, handlersをserverクレートに統合
   - trait abstractionの最小化
   - 具体的実装の優先

3. **3層クレート構成**
   ```
   domain/     # 純粋ドメインロジック（I/Oなし）
   server/     # 統合バックエンド（全ての非同期処理）
   web/        # Leptosフロントエンド（SSR + hydration）
   ```

### 重要な設計原則

- **型駆動設計**: Rustの型システムでビジネスルールを表現
- **所有権活用**: borrowingとmoveセマンティクスの最適利用
- **ゼロコスト抽象化**: 必要な場所でのみtraitを使用
- **コンパイル時安全性**: 実行時エラーの最小化

## 結果

### ポジティブな影響

- **パフォーマンス向上**: 不要な抽象化の除去
- **型安全性**: Rustの型システムを最大活用
- **メンテナンス性**: 明確な責任分離
- **WASM互換性**: フロントエンドでのI/O依存除去
- **デプロイ簡素化**: 単一serverバイナリ

### ネガティブな影響

- **学習コスト**: 新しいアーキテクチャパターンの習得
- **リファクタリング**: 既存コードの大幅変更
- **テストability**: モック作成の複雑化（具象依存）

### 代替案

1. **従来のClean Architecture継続**
   - トレードオフ: 冗長性とパフォーマンス低下
   - リスク: WASM互換性問題

2. **マイクロサービス分離**
   - トレードオフ: 運用複雑性とネットワークオーバーヘッド
   - リスク: デプロイ・監視の複雑化

3. **モノリス設計**
   - トレードオフ: 責任の混在と拡張性不足
   - リスク: 長期メンテナンス性の劣化

## 参考資料

- [Leptos Book - Architecture](https://leptos.dev/)
- [Rust RFC - Zero-cost abstractions](https://github.com/rust-lang/rfcs)
- [Domain-Driven Design in Rust](https://github.com/domain-driven-design/ddd-rust)
- [Clean Architecture vs Rust-first Design](https://blog.rust-lang.org/2023/12/21/impl-trait-captures.html)

---

**作成者**: Claude (AI Assistant)  
**作成日**: 2025-09-20  
**最終更新**: 2025-09-20