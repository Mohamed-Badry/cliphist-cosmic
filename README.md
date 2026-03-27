# cliphist-cosmic

A Wayland clipboard picker built with [libcosmic](https://github.com/pop-os/libcosmic), using [cliphist](https://github.com/sentriz/cliphist) for history storage.

- Paged list with inline image previews
- Live search with cached filtering
- Async clipboard activation — no UI blocking
- Optional Vim-style modal navigation (`--vim`)
- Mouse menu: reload, delete, or wipe history
- Configurable window size, page size, and preview limits

## Requirements

**Runtime:**
- `cliphist` — clipboard history daemon
- `wl-copy` from `wl-clipboard`
- A running Wayland session

**Build:**
- Rust (stable) and Cargo
- `just` *(optional, for helper recipes)*

## Installation

### From source

```bash
git clone https://github.com/Mohamed-Badry/cliphist-cosmic
cd cliphist-cosmic
just install
# or: cargo install --path .
```

The binary installs to `~/.cargo/bin/cliphist-cosmic`.

### From GitHub

```bash
cargo install --git https://github.com/Mohamed-Badry/cliphist-cosmic
```

## COSMIC Desktop Integration

Bind the picker to a global shortcut in **Settings → Keyboard → Shortcuts → Add Shortcut**:

| Field    | Value                                             |
|----------|---------------------------------------------------|
| Name     | Clipboard Manager                                 |
| Command  | `cliphist-cosmic`                                 |
| Shortcut | `Super + V` *(or any key you prefer)*            |

### Command presets

```bash
cliphist-cosmic                                  # default
cliphist-cosmic --vim                            # vim navigation
cliphist-cosmic --vim --width 650 --height 465   # horizontal layout
cliphist-cosmic --page-size 30 --image-height 60 # compact list
```

## CLI Options

```text
Usage: cliphist-cosmic [OPTIONS]

Options:
      --vim                            Enable Vim keybindings
      --width <WIDTH>                  Window width in pixels [default: 480]
      --height <HEIGHT>                Window height in pixels [default: 560]
      --page-size <PAGE_SIZE>          Number of items per page [default: 16]
      --image-height <IMAGE_HEIGHT>    Image preview height in pixels [default: 116]
      --preview-lines <PREVIEW_LINES>  Max preview lines for text entries [default: 4]
      --preview-chars <PREVIEW_CHARS>  Max preview characters for text entries [default: 280]
  -h, --help                           Print help
```

## Keybindings

### Default

| Key | Action |
|-----|--------|
| Type | Filter search |
| `↑` / `↓` | Move selection |
| `←` / `→` or `PageUp` / `PageDown` | Change page |
| `Enter` | Copy selected entry |
| `Delete` | Remove selected entry |
| `Ctrl+R` | Reload history |
| `Esc` | Close |

### Vim Mode (`--vim`)

The app starts in **Normal** mode. Typing `/` or `i` enters **Insert** mode (coupled to the search box).

| Key | Action |
|-----|--------|
| `i` / `/` | Enter Insert mode |
| `Esc` or `jk` | Leave Insert mode |
| `j` / `k` | Move selection |
| `h` / `l` | Change page |
| `y` | Copy selected entry |
| `d` | Delete selected entry |
| `r` | Reload history |
| `q` | Close |

Arrow keys, `Enter`, and page navigation remain available in all modes.

## Just Recipes

```bash
just dev             # run in debug mode with vim bindings
just run             # run in release mode
just run-vim         # run in release mode with vim bindings
just run-horizontal  # run in release mode with horizontal preset
just fmt             # format code
just lint            # run clippy lints
just test            # run tests
just build-release   # build release binary
just install         # install to ~/.cargo/bin
just uninstall       # remove from ~/.cargo/bin
```

## Project Notes

- See [CHANGELOG.md](./CHANGELOG.md) for implementation history.
