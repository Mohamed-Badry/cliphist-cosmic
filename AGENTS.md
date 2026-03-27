# cliphist-cosmic Handoff

## Current State

This repo is a `libcosmic` + `cliphist` Wayland clipboard picker implemented under `src/`.

Current UI/runtime shape:

- single-pane paged list
- search box at the top
- inline image previews inside the list
- fixed target size of `480x560` by default
- async copy/delete/image loading paths
- default startup as a normal undecorated COSMIC window
- optional startup as a Wayland layer surface via CLI

Current history flow:

- `cliphist list` loads entries
- text and HTML entries render as compact text cards
- `image/*` entries decode on the visible page only
- `wl-copy` is used to put a selected entry back on the clipboard
- `cliphist delete` is wired for delete
- `cliphist wipe` is available from the overflow menu

## Surface Modes

The app now supports two startup modes:

- `--surface window`
- `--surface layer`

### Window Mode

- implemented through the normal `cosmic::app::run(...)` window path in `src/main.rs`
- keeps the draggable top handle in `src/view.rs`
- still behaves like a normal toplevel window

### Layer Mode

- uses `.no_main_window(true)` in `src/main.rs`
- creates the real surface from `ClipboardApp::init(...)` in `src/app.rs`
- issues a raw Wayland `get_layer_surface(...)` task via the `iced`/`libcosmic` Wayland commands in `src/config.rs`
- hides the drag handle because layer surfaces are for placement, not normal mouse dragging

### Why The App Uses Two Modes

Wayland is the reason this is split.

Regular draggable toplevel windows are positioned by the compositor, not by the app. That means a normal window can be draggable, but it cannot reliably honor startup coordinates. Layer surfaces can request anchors and margins, so they can be placed near a requested edge or absolute offset, but they stop behaving like a normal movable window.

That is why the CLI is explicit instead of trying to make one surface type do both jobs.

## Placement CLI

Placement flags are only valid with `--surface layer`.

Supported preset positions:

- `top-left`
- `top-center`
- `top-right`
- `center-left`
- `center`
- `center-right`
- `bottom-left`
- `bottom-center`
- `bottom-right`

Coordinate behavior:

- `--x` and `--y` must be passed together
- `--x --y` override `--position`
- absolute coordinates are applied as top/left layer margins

Relevant code:

- CLI parsing: `src/main.rs`
- placement model: `src/config.rs`
- layer startup task: `src/config.rs`
- init sequencing: `src/app.rs`

## Important Implementation Details

- Filtered indices are cached in `self.filtered`.
- Page image loading is done by `load_visible_images()`.
- Async image results come back through `Message::PageImagesLoaded`.
- Image previews are only decoded for the current page.
- Page image handles, request tracking, and preview cache eviction now live in `src/image_state.rs`.
- Text previews are truncated by `compact_preview_text()`.
- Copy and delete operations run asynchronously.
- Layer mode closes on Wayland layer unfocus in `src/keyboard.rs`.
- Selection movement is modeled with an explicit enum instead of sentinel integers.
- Keyboard subscriptions are split into close events and mode-specific key mapping; there is no global shared vim-mode flag anymore.

## Key Files

- `src/main.rs`: CLI, PID toggle logic, startup settings, surface mode selection
- `src/config.rs`: runtime config, placement enums, layer-surface builder task
- `src/app.rs`: app state, startup init chain, async actions
- `src/image_state.rs`: page image state, preview cache reuse, cache eviction
- `src/view.rs`: main UI, drag handle, mode-specific root layout
- `src/keyboard.rs`: key subscriptions and global close behavior
- `README.md`: user-facing explanation and CLI documentation

## Current Test Status

Latest verified status:

- `cargo fmt` passed
- `cargo test` passed

Current tests cover:

- CLI parsing and placement validation
- placement preset mapping
- vim mode startup and transitions
- keyboard mapping and global bindings
- image preview cache reuse and eviction
- selection movement
- paging math
- preview truncation
- parsing/model behavior

## Practical Resume Point

If work continues on startup behavior, begin here:

- `src/main.rs`
- `src/config.rs`
- `src/app.rs`

Most likely next follow-ups:

1. Runtime-test layer placement presets on real COSMIC Wayland sessions.
2. Decide whether placement flags should auto-switch to layer mode instead of erroring.
3. Add output selection if multi-monitor targeting becomes necessary.
