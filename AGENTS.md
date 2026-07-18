# Overview
A lightweight terminal emulator built with Vulkan.

For task-specific guidance, refer to the skills in [.skills](.skills):
- [.skills/shaka-development.md](.skills/shaka-development.md) for work on the shaka terminal emulator.
- [.skills/zellij-bridge-development.md](.skills/zellij-bridge-development.md) for work on the Zellij bridge plugin.
- [.skills/environment-and-compatibility.md](.skills/environment-and-compatibility.md) for setup, dependencies, and compatibility checks.

# Repository Structure
- asura: a thin wrapper around the shell.
- profiler_core: a profiling tool for shalacritty; this is deprecated.
- renge: a thin compatibility layer for migrating from shalacritty to shaka; it is deprecated once migration is complete.
- shaka: the main product of this project and the terminal emulator itself.
- shaka_visualizer: a development tool for profiling shaka.
- term-gfx: a crate for validating Vulkan rendering; it is deprecated now that shaka development is the main focus.
- zazen: a multiplexer intended for cross-platform support; it is largely unnecessary now that tools like Zellij already cover similar functionality.
- zellij-shaka-bridge: a plugin that combines shaka background switching with Zellij tab functionality.

shaka and zellij-shaka-bridge are the main development targets.
Other crates are deprecated and generally do not require maintenance beyond compatibility.

# Build / Run
The main development targets are:
- shaka: the terminal emulator itself.
- zellij-shaka-bridge: the bridge plugin for Zellij.
- shalacritty: an older implementation that exists for compatibility; confirm the impact of any changes before modifying it.

Main implementation entry points are:
- shaka/src: primary source for the terminal emulator.
- zellij-shaka-bridge/src: primary source for the bridge plugin.

Common commands are:

```cmd
cargo build -p shaka
cargo run -p shaka
```

```cmd
cargo build -p zellij-shaka-bridge
```

If changes affect other crates in the workspace, build them individually as needed with `cargo build -p <crate>`.

# Coding Guidelines
Follow these guidelines during development:
- Run `cargo fmt` after making changes to format the code.
- If possible, also run `cargo clippy` to check for warnings.
- Avoid breaking changes and preserve compatibility with existing interfaces.
- Avoid modifying deprecated crates unless the change is necessary for compatibility or build reasons.
- When changing shalacritty, the older implementation, verify the scope of impact carefully before making changes.

# Prerequisites
Before working on this repository, make sure the required environment is available:
- A working Rust toolchain is installed.
- Any platform-specific dependencies required for Vulkan and the terminal stack are available.
- If you are working on macOS, confirm that the relevant system libraries and runtime requirements are satisfied.

# Recommended Workflow
When making changes:
1. Inspect the relevant source files in the affected crate first.
2. Make the change with minimal scope.
3. Run formatting and lint checks.
4. Run targeted tests for the affected crate if available.
5. Run the full test suite before finishing.

# Compatibility Check
After modifying the code, ensure compatibility is preserved by running the full test suite.

```cmd
cargo test
```