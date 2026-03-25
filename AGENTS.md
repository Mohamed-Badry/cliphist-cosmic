# cliprs Handoff

## Current State

This repo is currently a `libcosmic` + `cliphist` Wayland clipboard picker implemented in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs).

The current UI shape is:

- single-pane list layout
- search box at the top
- inline image previews inside the list
- fixed paging instead of rendering the entire history at once
- fixed target window size of `480x560`
- layer-shell popup setup instead of a normal app window

The current history flow is:

- `cliphist list` loads entries
- text and HTML entries render as compact text cards
- `image/*` entries decode on the visible page only
- `wl-copy` is used to put a selected entry back on the clipboard
- `cliphist delete` is wired for delete

## What Was Changed

- Removed the two-pane layout and moved images inline into the list.
- Reduced visible window size and set fixed size limits.
- Added cached filtered indices so search and paging stop rebuilding the world repeatedly.
- Changed image preview loading to page-local async tasks instead of synchronous decode on every selection movement.
- Reduced preview text size by truncating large clipboard text before layout.
- Kept keyboard navigation, reload, delete, and copy paths in code.

## Known Runtime Bugs

These are the current issues to fix next:

- The popup background is still transparent in practice.
- Clicking an entry hangs the app and it stops responding.
- `Esc` does not reliably close the popup.
- `Enter` does not reliably activate/copy the selected entry.

## Likely Causes

### 1. Transparent Background

The app is started with `.transparent(false)` in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L922), but the main content is just a plain container. It likely needs an explicit themed/background container layer instead of relying on the default surface fill.

Good first place to inspect:

- [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L386)
- `cosmic::widget::layer_container(...)` may be the right fix if it exists in this build

### 2. Clicking Entry Hangs

Click currently triggers immediate copy:

- [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L302)
- [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L605)

`copy_selected()` calls `decode_entry()` and then `wl-copy` synchronously on the UI path. That is the most likely reason the app freezes when an item is clicked.

Next fix should be:

- make click only select
- make `Enter` perform activation
- or move copy/decode into `Task::perform(...)`

### 3. Missing `Esc` Handling

There are two current close paths:

- `on_escape()` in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L198)
- unfocus handling in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L206)

`Esc` likely is not reaching `on_escape()` consistently for this layer-surface/search-input setup, or focus is being consumed by the text input. If needed, add an explicit keyboard match for `Named::Escape` inside the subscription handler.

### 4. Missing `Enter` Handling

There are two current activation paths:

- search input submit in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L350)
- keyboard subscription for `Named::Enter` in [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L227)

Because the subscription only handles keys when `status == event::Status::Ignored`, `Enter` may be consumed by the text input and never reach app-level activation.

Likely fixes:

- handle `Enter` from the search input differently
- or allow `Enter` in the subscription even when the input has focus
- or separate search submit from item activation

## Recommended Next Steps

1. Fix activation flow first.
2. Make click only change selection.
3. Make `Enter` do copy in an async task.
4. Add explicit `Escape` handling in the keyboard subscription.
5. Wrap the root content in an explicit opaque COSMIC background container.
6. After that, runtime-test on the real Wayland session for compositor behavior.

## Notes About Window Behavior

The popup is currently configured as a layer surface in:

- [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L737)

Current settings:

- `Layer::Overlay`
- `Anchor::TOP`
- `exclusive_zone = 0`
- fixed width and height
- top margin of `28`

This was intended to stop it from behaving like a normal tiled app window. If it still tiles or places badly on a given compositor, the next debug step is compositor-specific layer-shell behavior rather than normal `libcosmic` window flags.

## Current Test Status

These pass:

- `cargo fmt`
- `cargo test`

Current tests cover:

- parsing cliphist list lines
- HTML preview detection
- selection movement
- paging math
- preview truncation

## Important Implementation Details

- Filtered indices are cached in `self.filtered`.
- Page image loading is done by `load_visible_images()`.
- Async image results come back through `Message::PageImagesLoaded`.
- Image previews are only decoded for the current page.
- Text previews are truncated by `compact_preview_text()`.

## Practical Resume Point

Resume from [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L240) and [src/main.rs](/home/crim/Projects/cliprs/src/main.rs#L605).

The first high-value patch should be:

- stop copying on click
- make activation async
- fix `Esc` and `Enter`
- then make the root container explicitly opaque
