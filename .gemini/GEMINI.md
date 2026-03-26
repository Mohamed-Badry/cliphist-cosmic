# GEMINI

## Context & Status
- **Goal:** Resolve async blocking anti-patterns in `cliphist.rs` to prevent UI freezing.
- **Current Task:** Migrating `std::process::Command` usage to `tokio::process::Command` allowing concurrent image decoding.
- **Next Steps:** Requesting implementation plan review. After approval, update `Cargo.toml`, modify `cliphist.rs`, and test.

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
