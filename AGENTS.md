# Working on NSVim

A modal code editor. The core and all app state are Rust. The screens are TSX on
[NSH](https://nsh.dev), which draws with the native views of each platform. Read
`docs/architecture.md` before you change structure.

## Commands

```sh
nsh run                                       # the app in a window (NSH 0.7 or later)
nsh check                                     # cargo check, the nsvim-native types, tsc
nsh build                                     # the crate and the TSX bundle. Read its warnings
TMPDIR=~/.cache/nsvim-test-tmp cargo test     # every test: core, app, Headless
cargo test -p editor-core                     # the core only, fast, no window
cargo clippy --all-targets -- -D warnings     # CI runs this
cargo fmt                                     # rustfmt.toml: 140 columns
target/debug/nsvim --help                     # the command line
```

Never use `/tmp`. Point `TMPDIR` at a folder outside `/tmp` and outside the checkout.

## Check your work

NSH runs the real window headless. App tests in `ext/nsvim-native/tests/` build `App.tsx`, press keys and
views, and read texts, test ids and app state. Copy the helpers in `ext/nsvim-native/tests/common/mod.rs`
(`start`, `typ`, `type_text`, `write`, `scratch`). Give a view that a test finds a `testID` in
TSX or a `.test_id(..)` in Rust.

Headless tests prove behaviour, not the look. For a visible change, run the app (`nsh run`)
and look at it. A window run must use a scratch home and scratch config. The setup screen
writes to `~/.bashrc` and git's global config:

```sh
S=~/.cache/nsvim-scratch; mkdir -p $S/home
HOME=$S/home XDG_CONFIG_HOME=$S/config XDG_STATE_HOME=$S/state nsh run
```

## Where the UI lives

- Screens are TSX: `App.tsx` and `src/`. They import components and modules from
  `nsvim-native` (`ext/nsvim-native/src/exports`).
- Rust owns all state. A TSX screen keeps a part of it in a signal, from a module's `onChange`
  listener. It asks for changes through module functions. After a press, it calls
  `Shell.focusKeys()`, because the Rust `Workspace` view takes every key.
- The text area, the popups by the cursor, the diff review and the native context menu stay in
  Rust (`ext/nsvim-native/src/ui`), for speed on each key.
- Colours come only from theme tokens, as classes: `bg-muted`, `text-muted-foreground`,
  `text-mode-insert`, `code-*`. See `theme.json` and `ext/nsvim-native/src/ui/style.rs`. Write no raw
  colours.
- Corners are square: `rounded-(--radius)` (2px). Use checkboxes, not switches.
- A selected row shows its state with `aria-selected:` classes, and has no hover style.
- Interface copy follows the NSH Design Guides: sentence case for labels, hints and states.
  Write the product name as NSVim. Keep the casing of the `nsvim` binary, file names, commands (`:w`), key names, acronyms (UTF-8, LSP)
  and Vim's own text (`NORMAL`, `E37: ...`).

## Rules

- The core (`crates/editor-core`: buffer, editor, vim, classic, syntax) has no window code and
  no I/O. A keymap returns `Action`s. The app carries them out.
- No file I/O, process spawning or LSP traffic on the engine thread. Workers in
  `ext/nsvim-native/src/services` do it and send `Event`s back.
- Every edit goes through `Buffer` inside `begin`/`commit`: one command, one undo step.
- Change app state only through `Session::update`, or from TSX through `crate::bridge::send`
  and `crate::bridge::update`. A Rust view reads state only through a memo (`Session::view`),
  because a component body runs once.
- Keep files under 800 lines. Tests go in a sibling `*_tests.rs` file or in `ext/nsvim-native/tests/`.
- Text: ASD-STE100 Simplified Technical English in comments, docs and messages. Short
  sentences, active voice, no em dashes or semicolons in prose.
- Never add AI attribution to commits, code or docs.

## Talking to a running editor

`nsvim ctl` and `nsvim mcp` drive a running editor over its socket. See `docs/agents.md`.
