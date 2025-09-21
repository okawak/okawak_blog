# Architecture Decision Records (ADR)

このディレクトリには、okawak_blogプロジェクトのアーキテクチャ決定記録（ADR: Architecture Decision Records）を保管しています。

## ADRとは

ADRは、ソフトウェア開発プロジェクトにおける重要なアーキテクチャ決定を文書化するためのドキュメント形式です。各決定の背景、検討した選択肢、決定理由を記録し、将来の開発者が意思決定の経緯を理解できるようにします。

## ADRの命名規則

```
NNNN-短い説明.md
```

- `NNNN`: 4桁の連番（例: 0001, 0002, ...）
- 短い説明: ケバブケース（ハイフン区切り）

## ADRリスト

| ADR | 題名 | ステータス | 日付 |
|-----|------|------------|------|
| [0001](./0001-rust-first-architecture.md) | Rust-First アーキテクチャの採用 | Accepted | 2025-09-20 |
| [0002](./0002-domain-layer-purification.md) | ドメイン層の純粋化 | Accepted | 2025-09-20 |
| [0003](./0003-server-layer-integration.md) | サーバー層統合設計 | Accepted | 2025-09-20 |

## ADRステータス

- **Proposed**: 提案段階
- **Accepted**: 採用決定
- **Deprecated**: 非推奨
- **Superseded**: 他のADRに置き換え

## 新しいADRの作成

新しいADRを作成する際は、[template.md](./template.md)をベースにしてください。

```bash
cp docs/adr/template.md docs/adr/NNNN-新しい決定.md
```