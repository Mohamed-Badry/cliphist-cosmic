# Changelog

This file tracks the major changes made to the current `cliphist-cosmic` tree compared with the earlier prototype.

## Surface Modes And Placement

- Added explicit startup surface selection with `--surface window|layer`.
- Kept `window` mode as the default so the picker still behaves like a normal draggable COSMIC window.
- Added `layer` mode for Wayland users who care more about startup placement than mouse drag.
- Added placement presets with `--position`:
  - `top-left`
  - `top-center`
  - `top-right`
  - `center-left`
  - `center`
  - `center-right`
  - `bottom-left`
  - `bottom-center`
  - `bottom-right`
- Added absolute placement through `--x` and `--y`, with `--x --y` overriding `--position`.
- Made placement flags valid only for `--surface layer`.
- Added startup logic that disables the normal main window and creates a Wayland layer surface only when layer mode is requested.
- Hid the drag handle in layer mode and kept it in window mode.
- Added layer-unfocus close handling so the layer-surface picker still behaves like a transient launcher.
- Documented why this is split into two modes: Wayland compositors control placement for normal toplevel windows, while layer surfaces can request anchored placement but do not behave like normal movable windows.

## UI And Windowing

- Replaced the older two-pane layout with a single-pane paged list.
- Moved image previews inline into the main list instead of showing them in a separate side panel.
- Added CLI configuration for window size, page size, and preview limits.
- Implemented a custom selection style to match the COSMIC launcher (secondary color highlight, transparent background).
- Added a 3-dot overflow menu for advanced actions (Reload, Delete, Wipe All History).
- Fixed item text and icon colors to use standard interface tints instead of accent colors.
- Switched from a centered layer-shell popup to a fixed-size undecorated toplevel window.
- Added a small draggable top handle so the window can be moved with the mouse.
- Moved the vim mode indicator into the main info row to reduce wasted header space.
- Implemented a PID-based toggle mechanism to allow opening/closing the app with the same shortcut.

## Clipboard And Data Flow

- Kept `cliphist list` as the source of clipboard history.
- Kept `cliphist delete` for removing entries.
- Added `cliphist wipe` support via the advanced menu.
- Kept `wl-copy` for copying the selected entry back to the clipboard.
- Made activation/copy happen asynchronously instead of blocking the UI path.
- Added cached filtered indices so search and paging stop rebuilding the world repeatedly.
- Limited image decoding to the visible page.
- Added async page image loading through `PageImagesLoaded`.
- Truncated large text previews before layout so oversized clipboard entries do not bloat rendering.

## Keyboard And Vim Mode

- Added `--vim` startup support.
- Reconciled vim behavior across `keyboard`, `app`, `view`, and `vim` modules.
- Moved key handling away from physical key codes and onto `iced` logical keys.
- Made vim mode start in `Normal` mode.
- Coupled `Insert` mode directly to focus on the search box.
- Made `Esc` leave `Insert` mode instead of closing the app when the search box is focused.
- Kept `jk` as an alternate escape from `Insert`.
- Kept `Left` / `Right` page navigation global regardless of mode.
- Added a visible mode indicator in the UI.

## Reliability And Tests

- Added broader tests around vim mode startup, focus selection, escape behavior, and `jk` timing.
- Added keyboard mapping tests for logical key handling and global bindings.
- Added helper tests for paging, preview truncation, HTML detection, and parsing/model behavior.
- Current test suite passes with `cargo test`.

## Build And Tooling

- Added a `justfile` for local build, run, and install flows.
- Expanded the `justfile` with `lint`, `dev`, and `run-horizontal` recipes.
- Restructured `README.md` to improve clarity, with tables for keybindings and global shortcut setup instructions.
- Changed the default install workflow to Cargo-managed installs so local development matches future Git-based Cargo installs.
- Added `repository` and `homepage` metadata to `Cargo.toml`.
- Added release profile tuning in `Cargo.toml` for smaller optimized binaries.
