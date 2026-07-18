---
name: environment-and-compatibility
description: Use this skill for environment setup, dependency checks, and compatibility validation.
---

# Environment and Compatibility

## When to use this skill
Use this when setting up the workspace, validating build prerequisites, or checking compatibility before finishing a change.

## Prerequisites
- A working Rust toolchain is installed.
- Platform-specific dependencies required for Vulkan and the terminal stack are available.
- On macOS, confirm that the relevant system libraries and runtime requirements are satisfied.

## Validation steps
- Run formatting with `cargo fmt` after making code changes.
- Run `cargo clippy` when possible to check warnings.
- Run targeted tests for the affected crate if available.
- Run the full test suite before finishing changes.

```cmd
cargo test
```

## Change policy
- Avoid breaking changes and preserve existing interfaces.
- Avoid modifying deprecated crates unless the change is necessary for compatibility or build reasons.
- When changing [shalacritty/src](../shalacritty/src), confirm the impact carefully before editing.
