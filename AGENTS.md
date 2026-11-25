# Working on nsvim

A modal code editor in Rust. Its UI is [NSH](https://nsh.dev), which draws with the native
views of each platform. Read `docs/architecture.md` before you change structure.

## Commands

```sh
nsh run                                       # the app in a window (NSH 0.7 or later)
cargo build                                   # debug: target/debug/nsvim
TMPDIR=~/.cache/nsvim-test-tmp cargo test     # every test: core, app, Headless
cargo test -p editor-core                     # the core only, fast, no window
cargo clippy --all-targets -- -D warnings     # CI runs this
cargo fmt                                     # rustfmt.toml: 140 columns
```

Never use `/tmp`. Point `TMPDIR` at a folder outside `/tmp` and outside the checkout.

## Check your work without a window

NSH runs the real window headless. App tests in `nsh/tests/` press keys and views, and read
texts, test ids and app state. Copy the helpers in `nsh/tests/common/mod.rs` (`start`, `typ`,
`type_text`, `write`, `scratch`). Give a view that a test finds a `.test_id(..)`.

Headless tests prove behaviour, not the look. For a visible change, run the app (`nsh run`)
and look at it.

## Rules

- The core (`crates/editor-core`: buffer, editor, vim, classic, syntax) has no window code and
  no I/O. A keymap returns `Action`s. The app carries them out.
- No file I/O, process spawning or LSP traffic on the engine thread. Workers in
  `nsh/src/services` do it and send `Event`s back.
- Every edit goes through `Buffer` inside `begin`/`commit`: one command, one undo step.
- Change app state only through `Session::update`. A view reads state only through a memo
  (`Session::view`), because a component body runs once.
- Colours come only from theme tokens (`nsh/src/ui/style.rs`). Write no raw colours in views.
- Keep files under 800 lines. Tests go in a sibling `*_tests.rs` file or in `nsh/tests/`.
- Text: ASD-STE100 Simplified Technical English in comments, docs and messages. Short
  sentences, active voice, no em dashes or semicolons in prose.
- Never add AI attribution to commits, code or docs.

## Talking to a running editor

`nsvim ctl` and `nsvim mcp` drive a running editor over its socket. See `docs/agents.md`.
