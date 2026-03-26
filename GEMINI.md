# GEMINI

## Context & Status
- **Goal:** Implement cliphist-cosmic functionality according to AGENTS.md, followed by architectural refactoring.
- **Current Task:** Refactoring monolithic `src/main.rs` code into modular architecture (`models`, `utils`, `app`, `cliphist`).
- **Next Steps:** Review implementation plan with user, create modules, resolve import namespaces, and verify compilation.

## Technical Decisions (ADRs)
- **Stack:** Rust (libcosmic + iced)
- **Why:** Existing cliphist-cosmic implementation for system integration.

## Development Commands
- **Build:** `cargo build`
- **Test:** `cargo test`
- **Lint/Format:** `cargo fmt && cargo clippy`

## Architecture Overview
- **Entry Point:** `src/main.rs`
- **Key Directories:**
  - `src/`: Core logic

## Role & Style
- **Persona:** Senior Rust developer using libcosmic
- **Priorities:** Maintainability, safety, precise asynchronous state management.
