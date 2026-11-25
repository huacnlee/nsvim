# Development

## Build and run

```sh
nsh run                  # the app in a window, rebuilt on each change (NSH 0.7 or later)
cargo run -p nsh-app-editor -- path/to/file.rs   # the same app, without the NSH CLI
cargo build --release    # target/release/nsvim
nsh build --release -o target/release/nsh   # its interface, next to it (docs/install.md)
```

`nsh.json` at the root names the app's crate (`"rust": "nsh"`) and its root component
(`"root": "App"`). Install the NSH CLI from [nsh.dev](https://nsh.dev/docs/installation).

Linux needs GTK 4.14 or later (`libgtk-4-dev` on Debian and Ubuntu).

## Code layout

```text
Cargo.toml            the workspace: nsh and crates/editor-core
nsh.json              the NSH app: its crate, root component and theme
crates/editor-core/   the core (library `editor`): no window code, no I/O
  src/buffer.rs       rope, undo, marks, saves
  src/editor.rs       Editor, Doc, Mode, Action, KeyInput
  src/vim/            the Neovim keymap, Ex commands, search, regex
  src/classic/        the classic keymap and multiple cursors
  src/syntax/         tree-sitter objects, motions and highlight queries
nsh/                  the app (binary `nsvim`)
  src/lib.rs          the root component and the window
  src/store.rs        Session: the app state, updates and effects
  src/model.rs        the text area's frame, from the Editor
  src/app/            state, key routing, mouse, pickers, Settings, setup, LSP glue
  src/ui/             views and theme tokens
  src/services/       workers: io, index, git, grep, watch, lsp, probe, ipc
  src/ipc/            nsvim ctl and nsvim mcp
  tests/              Headless tests of the real window
docs/                 this documentation
assets/               the icon and the Linux desktop entry
website/              the website (Astro)
```

Keep files under 800 lines, and put tests in a sibling `*_tests.rs` file or in `nsh/tests/`.

## Tests

```sh
TMPDIR=~/.cache/nsvim-test-tmp cargo test     # every test: core, app, Headless
cargo test -p editor-core                     # the core only, fast, no window
cargo clippy --all-targets -- -D warnings     # CI runs this
cargo fmt
```

Point `TMPDIR` at a folder outside `/tmp` and outside the checkout. Tests make project folders
in it.

The app tests in `nsh/tests/` start the real window in NSH's Headless engine, with no display.
They press keys and views, and they read texts, test ids and app state. For example:

```rust
let mut app = start(&file);
typ(&mut app, &["j", "d", "d"]);
app.wait_for("text:/let b/")?;
```

The helpers are in `nsh/tests/common/mod.rs`. They keep `HOME`, `XDG_CONFIG_HOME` and
`XDG_STATE_HOME` in a scratch folder, so no test writes to your home folder.

The bench tests print timings and check a budget. Run them in an optimized build:

```sh
TMPDIR=~/.cache/nsvim-test-tmp cargo test --profile quick -p nsh-app-editor \
  --test flows_bench -- --ignored --nocapture --test-threads=1
```

## Releases

Push a tag that starts with `v`:

```sh
git tag v0.4.0
git push origin v0.4.0
```

The tag must be the version in `nsh.json` and `nsh/Cargo.toml`, with a `v` in front.

`.github/workflows/release.yml` builds `nsvim` for Linux, macOS and Windows. Each job compiles
App.tsx with `nsh build --release`. `scripts/package-release.py` packs the program with its
interface (`nsh/bundle.nshc`) into the archive that `install.sh` and `install.ps1` read. The
publish job joins the checksums into one `SHA256SUMS` and attaches everything to a GitHub release.
[docs/install.md](install.md) lists the archives.

`.github/workflows/ci.yml` runs the format check, clippy and the tests on every push and pull
request. It also tests the installers against local release fixtures:

```sh
TMPDIR=~/.cache/nsvim-test-tmp scripts/test-install.sh
```

`scripts/test-install.ps1` does the same for `install.ps1` on Windows.
