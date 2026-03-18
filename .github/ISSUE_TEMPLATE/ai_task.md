---
name: "🤖 AI Coding Task"
about: Gemini CLI にコーディングを依頼する
title: "[AI] "
labels: ["gemini-run"]
---

##  Goal
ex. Add files

## Scope
- **Primary:** `crates/shaka/src/input.rs`
- **Secondary:** `crates/shaka/src/config.rs`
- **Avoid:** `vulkan-render/` (Ignore them)

## Requirements & Constraints
1. `architecture.md` のイベントループ設計に従うこと。
2. 新しい依存ライブラリ（crate）を追加しないこと。
3. 既存のユニットテストが通ることを確認すること。

## Context Hints
- `alacritty_terminal` の `Action` 型を拡張する必要があります。
- 以前 `baby-rs` で実装したネットワークエラーハンドリングの手法を参考にしてください。

## Definition of Done
- [ ] 修正後のコード
- [ ] 影響範囲の短いサマリー
