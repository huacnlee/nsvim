# nsvim on NSH

This crate draws nsvim with [NSH](https://nsh.dev). NSH uses the native views of each
platform: GTK 4 on Linux, AppKit on macOS and WinUI on Windows. The core does not change. It is
the crate `editor-core` in `../crates/editor-core`.

## Run it

```sh
cargo run -- path/to/file.rs        # this machine: Linux, macOS or Windows
TMPDIR=~/.cache/editor-test-tmp cargo test
cargo clippy --all-targets -- -D warnings
```

This crate is its own Cargo workspace. The Rune patches in the root manifest do not apply to it.

## How it works

| File | Holds |
| --- | --- |
| `src/store.rs` | `Session`: the core `Editor` in a `stored` cell, a revision signal, and the `frame` memo. Keys, clicks, the wheel, open and save. |
| `src/model.rs` | `frame`: what the screen shows, from the `Editor`. Plain Rust, with no NSH code. |
| `src/text_area.rs` | One slot per visible row. Each slot reads only its row of the frame. The cursor is a view on top. |
| `src/chrome.rs` | The status line and the command line. |
| `src/keys.rs` | NSH `KeyEvent` to core `KeyInput`. |
| `src/theme.rs` | `theme.json` as tokens, the code font, the token for each kind of code. |

A key goes to the core keymap. The keymap changes the `Editor`, and `Session::edit` bumps the
revision. The `frame` memo runs again. Each row slot compares its row, and only the rows that
changed build again. Syntax colours come from the tree-sitter highlight queries of the
grammars. They run only over the lines on screen (`editor::syntax::highlight`).

File reads and writes run in `blocking`. Their results come back in a `spawn` on the engine
thread.

## Status

Done: open, Vim and classic keys, cursor, Visual selection, scrolling (keys and wheel), click to
place the cursor, `:w`, `:q`, colours for 13 languages, light and dark theme.

Not done: tabs, file tree, finder and pickers, leader menu, Settings, setup, git signs, LSP,
search highlights, wrap, the control socket (`nsvim ctl`, `nsvim mcp`), sessions, swap files,
and IME input.

## Known limits of NSH that the port works around

- NSH 0.7.0 or later. On 0.6.1, a Rust root with a `window()` opens two windows on Linux.
- On Linux, `focus()` works only after the window shows. The app asks again until the view has
  the focus. A click in the text gives the focus back.
- In Headless tests, a `window()` root shows only on a desktop OS: the tests set `Os::Linux`.
