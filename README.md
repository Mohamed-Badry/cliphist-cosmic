# cliprs

A blazingly fast Wayland clipboard manager frontend built on `libcosmic` and `iced`. It provides a gorgeous, modern, unified layout designed to easily traverse, preview, and activate clipboard history streams generated from `cliphist`.

`cliprs` is designed for ultimate keyboard ergonomics, featuring standard search filters along with native **Vim Keybindings**. 

## Usage

Simply run `cliprs`. The window will automatically center on the active monitor as a polished layer-shell surface without distracting compositor borders. 

### CLI Arguments

`cliprs` accepts standard CLI flags for configuration:

| Flag | Description |
|---|---|
| `--vim` | Enables Vim Mode. Transforms standard clipboard execution into a modal experience featuring explicit `Normal` and `Insert` contexts. |
| `--help`, `-h` | Prints help information and all available arguments. |

## Keybindings & Navigation

`cliprs` has two core control schemas.

### Default Mode
By default, the clipboard remains heavily reliant on text-input filtering.

- **Any text**: Types into the active search bar, instantly dropping the visual list to matched results.
- **Up / Down Arrow**: Shifts the active selection smoothly.
- **Left / Right Arrow**: Flips pages regardless of whether the search box has focus.
- **PageUp / PageDown**: Shifts the list up or down by 16 items at a time without breaking search focus.
- **Enter**: Instantly copies the selected item and securely closes the board.
- **Ctrl+R**: Reloads standard history without dropping search variables.
- **Delete**: Wipes the selected string precisely out of your `cliphist` record globally.
- **Escape**: Closes the application.

### Vim Mode (`--vim` flag)

When launched natively with `--vim`, the clipboard assumes a **Modal Context** standard to terminal multiplexers and vim buffers. The app initiates in **Normal Mode**, and **Insert Mode** is coupled directly to focus on the search box.

#### Insert Mode
Your keystrokes directly map into the search query textbox while it is focused.
- **`<Esc>`**: Drops the app actively into **Normal Mode**, blurring the cursor.
- **`jk`**: Rapidly typing `jk` (within 300ms delay) will instantly exit the textbox, deleting the sequence and flawlessly dumping you into **Normal Mode**. 

#### Normal Mode
Global layout bindings. `j`, `k`, `r`, `d`, etc. no longer print into the search box, and instead manipulate the Wayland clipboard state directly!
- **`j` / `k`**: Map selection visually Down and Up identically to arrows.
- **`h` / `l`**: Map selection precisely across 16-item page barriers (h = Previous Page, l = Next Page).
- **`d`**: Wipes the selection from clipboard history.
- **`r`**: Reloads and parses cliphist.
- **`y`**: "Yanks" the selection instantly simulating `Enter` copying execution.
- **`i` / `/`**: Pushes the cursor explicitly back into the search box, returning securely into **Insert Mode**.
- **`q`**: Shuts down the Wayland application cleanly.
- **Left / Right / Up / Down / Enter / Del / Esc**: Standard mappings remain globally functional throughout both modes.
