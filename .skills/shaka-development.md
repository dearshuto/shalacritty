---
name: shaka-development
description: Use this skill when implementing or debugging the shaka terminal emulator.
---

# Shaka Development

## When to use this skill
Use this when changing the terminal emulator implementation in [shaka/src](../shaka/src) or when investigating behavior of the main shaka product.

## Primary scope
- Main crate: [shaka/Cargo.toml](../shaka/Cargo.toml)
- Main implementation entry point: [shaka/src](../shaka/src)
- Related compatibility layer: [renge/src](../renge/src)

## Build and run
```cmd
cargo build -p shaka
cargo run -p shaka
```

## Working guidelines
- Inspect the relevant source files in [shaka/src](../shaka/src) before editing.
- Keep changes focused and avoid unrelated refactors.
- Preserve existing interfaces where possible.
- If the change affects shared behavior, verify the impact on related crates.
