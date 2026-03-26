# cliphist-cosmic

`cliphist-cosmic` is a Wayland clipboard picker built with `libcosmic` and backed by `cliphist`.

The current app is a fixed-size clipboard browser with:

- a single-pane paged list
- a search box at the top
- inline image previews
- async clipboard activation and page-local image decoding
- optional vim-style modal navigation with `--vim`
- a small draggable handle so the undecorated window can still be moved

The current implementation uses a normal fixed-size toplevel window instead of layer-shell. That is the tradeoff that makes mouse dragging possible.

## Runtime Dependencies

`cliphist-cosmic` expects these tools to be available at runtime:

- `cliphist`
- `wl-copy` from `wl-clipboard`
- a Wayland session / compositor

## Build Dependencies

To build from source you need:

- Rust and Cargo
- `just` if you want the helper recipes in the `justfile`

## Rust Dependencies

Direct Cargo dependencies in this repo:

- `clap`
- `libcosmic`

`libcosmic` brings in the `iced` stack used for UI and input handling.

## Usage

Run the app directly:

```bash
cargo run
```

Run with vim bindings enabled:

```bash
cargo run -- --vim
```

If you use `just`:

```bash
just run
just run-vim
```

## Installation

The preferred install path for this repo now follows Cargo-managed binaries instead of copying into `~/.local/bin`.

From the repository:

```bash
just install
```

That installs `cliphist-cosmic` the same way `cargo install --path .` does, which means it goes to Cargo's bin root by default, typically `~/.cargo/bin`.

For local workflow testing with an isolated install root:

```bash
cargo install --path . --force --locked --root /tmp/cliphist-cosmic-root
cargo uninstall --root /tmp/cliphist-cosmic-root cliphist-cosmic
```

Once the repo is pushed to GitHub, users can install directly from the repository source with Cargo:

```bash
cargo install --git <repo-url>
```

If a GitHub release installer script is added later, that should be a separate binary-install path for release artifacts instead of replacing the Cargo-managed install flow.

## Just Recipes

The repo includes a deliberately small `justfile`:

- `just fmt`
- `just test`
- `just build-release`
- `just run`
- `just run-vim`
- `just install`
- `just uninstall`

## Keybindings

### Default Mode

- typing filters the search box
- `Up` / `Down` move selection
- `Left` / `Right` change page
- `PageUp` / `PageDown` change page
- `Enter` copies the selected entry
- `Ctrl+R` reloads history
- `Delete` removes the selected entry from `cliphist`
- `Esc` closes the app

### Vim Mode

Launch with `--vim` to enable modal behavior.

- app starts in `Normal` mode
- `Insert` mode is coupled to search-box focus
- `i` or `/` enters `Insert`
- `Esc` leaves `Insert` and returns to `Normal`
- `jk` also leaves `Insert` when typed quickly
- `j` / `k` move selection
- `h` / `l` change page
- `y` activates the current selection
- `d` deletes the current selection
- `r` reloads history
- `q` closes the app
- arrow keys, `Enter`, and page navigation remain globally available

## Current Behavior Notes

- Window size is fixed at `480x560`.
- The window is undecorated and movable via the small top drag handle.
- Search filtering uses cached filtered indices.
- Image previews are decoded only for the visible page.
- Clipboard activation runs asynchronously so clicking and pressing `Enter` do not block the UI thread.

## Project Notes

- [CHANGELOG.md](./CHANGELOG.md) tracks the major implementation changes made so far.
- Before publishing the repo, add a license and repository/homepage metadata once those are finalized.
