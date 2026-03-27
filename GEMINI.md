# GEMINI

## Context & Status

- **Goal:** Maintain `cliphist-cosmic` as a Wayland clipboard picker with both normal-window and fixed-placement startup options.
- **Current State:** The app defaults to a normal undecorated COSMIC window with mouse drag, and can optionally start as a Wayland layer surface with CLI placement flags.
- **Latest Completed Work:** Added `--surface window|layer`, `--position`, and `--x/--y`, then refactored the internal app architecture to isolate image preview state and remove keyboard global state.

## Technical Decisions

- **Stack:** Rust + `libcosmic` + `iced` + `cliphist`
- **Windowing Decision:** Keep two surface modes instead of forcing one implementation.
- **Why:** On Wayland, regular draggable toplevel windows do not get reliable app-controlled startup coordinates. Layer surfaces can request anchored placement, but they are not normal draggable windows. The dual-mode CLI is the explicit compromise required by Wayland.
- **Placement Rule:** `--position`, `--x`, and `--y` apply only to `--surface layer`.
- **Coordinate Rule:** `--x --y` override preset placement.

## Architecture Overview

- **Entry Point:** `src/main.rs`
- **Config / Placement Model:** `src/config.rs`
- **App State / Startup Sequencing:** `src/app.rs`
- **Image Preview State / Cache:** `src/image_state.rs`
- **UI Layout:** `src/view.rs`
- **Keyboard / Close Semantics:** `src/keyboard.rs`
- **Selection Model:** `src/messages.rs` and `src/utils.rs` now use an explicit selection-movement enum instead of integer sentinels.

## Runtime Notes

- `window` mode uses the standard `cosmic::app::run(...)` main window.
- `layer` mode sets `.no_main_window(true)` and creates the real surface from app init using raw Wayland layer-surface commands.
- The drag handle is shown only in `window` mode.
- Layer mode closes on layer-surface unfocus.
- Image previews are decoded only for the visible page, and preview cache reuse/eviction is handled outside `ClipboardApp` in `src/image_state.rs`.
- Keyboard subscriptions are composed from separate close-event and key-event listeners; there is no shared global vim-mode toggle in the keyboard layer.

## Development Commands

- **Build:** `cargo build`
- **Test:** `cargo test`
- **Format:** `cargo fmt`
- **Lint:** `cargo clippy`

## Current Verification Status

- `cargo fmt` passed
- `cargo test` passed after the internal architecture refactor
- `cargo clippy -- -D warnings` passed

## Likely Next Steps

1. Validate layer placement behavior on a live COSMIC Wayland session.
2. Tune default margins or presets if compositor behavior looks awkward.
3. Add output selection if multi-monitor placement becomes a requirement.
