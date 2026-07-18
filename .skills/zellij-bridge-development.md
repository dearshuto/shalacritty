---
name: zellij-bridge-development
description: Use this skill when working on the Zellij bridge plugin for shaka.
---

# Zellij Bridge Development

## When to use this skill
Use this when changing the integration plugin in [zellij-shaka-bridge/src](../zellij-shaka-bridge/src) or when debugging interactions with Zellij.

## Primary scope
- Main crate: [zellij-shaka-bridge/Cargo.toml](../zellij-shaka-bridge/Cargo.toml)
- Main implementation entry point: [zellij-shaka-bridge/src](../zellij-shaka-bridge/src)

## Build and run
The bridge crate has its own [.cargo/config.toml](../zellij-shaka-bridge/.cargo/config.toml), so run commands from the crate directory to ensure the correct Cargo configuration is applied.

```cmd
cargo build -p zellij-shaka-bridge
```

## Working guidelines
- Review the plugin entry points in [zellij-shaka-bridge/src](../zellij-shaka-bridge/src) before making changes.
- Keep the plugin behavior consistent with the expected Zellij integration contract.
- Validate that any change does not break the bridge contract or plugin compatibility.
