---
name: "🤖 AI Coding Task"
about: Gemini CLI にコーディングを依頼する
title: "[AI] "
labels: ["gemini-fix"]
---

##  Goal
ex. Add files

## Scope
- **Primary:** `shaka/src/input.rs`
- **Secondary:** `shaka/src/config.rs`
- **Avoid:** `vulkan-render/` (Ignore them)

## Requirements & Constraints
1. Strictly adhere to the design principles specified in `architecture.md`.
2. Do not introduce any new external dependencies (crates).
3. Ensure that all existing unit tests pass successfully after your changes.
4. Refrain from modifying or adding any existing test cases.

## Context Hints
- `alacritty_terminal` の `Action` 型を拡張する必要があります。
- 以前 `baby-rs` で実装したネットワークエラーハンドリングの手法を参考にしてください。

## Definition of Done
- [ ] Refactored and newly implemented code.
- [ ] A concise summary of the impact and changes.
