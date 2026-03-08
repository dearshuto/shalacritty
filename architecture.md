# Architecture: Terminal Ecosystem (asura, shaka, zazen)

## Contents

- [1. Overview](#1-overview)
- [2. Component Structure & Executables](#2-component-structure--executables)
  - [[Backend] asura](#backend-asura)
  - [[Frontend] shaka](#frontend-shaka)
  - [[Frontend] zazen](#frontend-zazen)
- [3. Data Flow](#3-data-flow)
- [4. Development Principles for AI](#4-development-principles-for-ai)

## 1. Overview

This project provides a terminal ecosystem: a low-level terminal management
crate, a GPU-accelerated terminal emulator, and a multiplexer frontend.

## 2. Component Structure & Executables

Each crate has its own implementation. Everything is written in Rust.

### [Backend] asura

- **Role:** terminal multiplexer backend
- **Core Library:**
  [alacritty_terminal](https://github.com/alacritty/alacritty/tree/master/alacritty_terminal)
- **Key Focus:** abstraction

### [Frontend] shaka

- **Role:** terminal emulator
- **Graphics API:** Vulkan 1.3
- **Mechanism:** Bridges window events and asura. It also renders contents from
  asura.

Note: The shalacritty in this repository is obsolete because it uses wgpu
instead of Vulkan directly. However, shaka is a rewritten rendering system using
Vulkan. So they share the same software design.

### [Frontend] zazen

- **Role:** multiplexer interface inspired by tmux and zellij
- **UI Framework:** `ratatui`
- **Mechanism:** Bridges window events and asura. It also renders contents from
  asura.
- **Responsibility:** TUI logic collaborating with asura: window splitting, tab
  management, status line drawing.

## 3. Data Flow

1. **Input:** `zazen` (TUI) or `shaka` (GUI) handle user input.
2. **Logic:** `asura` converts them into actual actions, connecting the shell
   and application through PTY.
3. **Update:** update states in `asura`
4. **Rendering:**
   - TUI mode: `zazen` renders contents using `ratatui` from `asura`
   - GUI mode: `shaka` renders contents using Vulkan from `asura`

## 4. Development Principles for AI

- **Cross-Crate Consistency:** Ensure compatibility across all crates in this
  repository when changing the protocol or shared data structures.
- **Performance:** Choose the best approach using asynchronous functions.
- **Vulkan Safety:** Ensure cross-platform compatibility when using Vulkan.
