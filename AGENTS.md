# Working on editor

A modal code editor in Rust on [Rune](https://github.com/anomalyco/rune). Read
`docs/architecture.md` before changing structure.

## Commands

```sh
cargo build                                   # debug
cargo test                                    # every test: core, tree-sitter, app, IPC
cargo test --lib                              # the core only, fast, no GPU
cargo clippy --all-targets -- -D warnings     # CI runs this
cargo fmt                                     # rustfmt.toml: 140 columns
```

## Check your work without a window

Rune renders headless. Use it to see a change instead of guessing:

```sh
target/debug/editor --list-scenes
target/debug/editor --snap /tmp/out.png --scene finder --size 1440x900 --at 0.5
target/debug/editor --scene editor --script keys.json --snap /tmp/out.png --at 3
target/debug/editor --scene editor --tree /tmp/tree.json     # the element tree as JSON
```

A script is `{"events": [{"t": 0.5, "type": "type", "text": "dd"}, {"t": 1, "type": "key",
"name": "Escape"}]}`. Add a scene in `src/scenes.rs` for new UI. App tests in
`src/app/tests.rs` drive the real view with keys and clicks; copy their `run`, `typ` and `ask`
helpers.

## Rules

- The core (`src/lib.rs`: buffer, editor, vim, classic, syntax) has no window code and no I/O.
  A keymap returns `Action`s; the app carries them out.
- No file I/O, process spawning or LSP traffic on the UI thread. Workers in `src/services` do
  it and send `Event`s back.
- Every edit goes through `Buffer` inside `begin`/`commit`: one command, one undo step.
- Keep files under 800 lines. Tests go in a sibling `tests.rs` or `*_tests.rs`.
- Text: ASD-STE100 Simplified Technical English in comments, docs and messages. Short
  sentences, active voice, no em dashes or semicolons in prose.
- Never add AI attribution to commits, code or docs.

## Talking to a running editor

`nsvim ctl` and `nsvim mcp` drive a running editor over its socket. See `docs/agents.md`.
