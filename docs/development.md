# Development

## Build and run

```sh
nsh run                  # the app in a window. TSX edits apply at once (NSH 0.7 or later)
nsh check                # cargo check, the nsvim-native types, and tsc over the TSX
nsh build                # the crate and the TSX bundle. Read its warnings
cargo build --release    # target/release/nsvim
nsh build --release -o target/release/nsh   # its interface, next to it (docs/install.md)
```

`nsh.json` lists the app's Rust as a path dependency (`ext/nsvim-native`), and `App.tsx` is the
entry. Install the NSH CLI from [nsh.dev](https://nsh.dev/docs/installation).

A window run uses your real home folder, and the setup screen writes to `~/.bashrc` and git's
global config. For tests by hand, use a scratch home:

```sh
S=~/.cache/nsvim-scratch; mkdir -p $S/home
HOME=$S/home XDG_CONFIG_HOME=$S/config XDG_STATE_HOME=$S/state nsh run
```

Linux needs GTK 4.14 or later (`libgtk-4-dev` on Debian and Ubuntu).

## Code layout

```text
Cargo.toml            the workspace: ext/nsvim-native and ext/editor-core
nsh.json              the NSH app: its theme and its Rust dependency
nsh.lock              the resolved dependencies, written by nsh
theme.json            the theme tokens
App.tsx               the entry: the window and its screens
src/                  the TSX screens: tabs, tree, status and command lines, overlays,
                      Settings, setup, shared controls
ext/editor-core/      the core (library `editor`): no window code, no I/O
  src/buffer.rs       rope, undo, marks, saves
  src/editor.rs       Editor, Doc, Mode, Action, KeyInput
  src/vim/            the Neovim keymap, Ex commands, search, regex
  src/classic/        the classic keymap and multiple cursors
  src/syntax/         tree-sitter objects, motions and highlight queries
ext/nsvim-native/     the app's nshm (binary `nsvim`)
  src/main.rs         the program: ctl, mcp, --wait, and the launch
  src/lib.rs          the crate's exports and the launch
  src/store.rs        Session: the app state, updates and effects
  src/bridge.rs       the join to TSX: listeners and commands
  src/exports/        nsvim-native: Workspace, TextArea, Icon and the area modules
  src/model.rs        the text area's frame, from the Editor
  src/app/            state, key routing, mouse, pickers, Settings, setup, LSP glue
  src/ui/             Rust views (text area, popups, diff review, menu), theme tokens
  src/services/       workers: io, index, git, grep, watch, lsp, probe, ipc
  src/ipc/            the ctl and mcp commands
  tests/              Headless tests of the real window
  web/                the web build (build.sh, the demo, libc)
docs/                 this documentation
assets/               the icon and the Linux desktop entry
website/              the website (Astro)
```

Keep files under 800 lines, and put tests in a sibling `*_tests.rs` file or in `ext/nsvim-native/tests/`.

## Look and copy

- The program is `nsvim`, in lower case.
- Colours come only from theme tokens (`theme.json`, `ext/nsvim-native/src/ui/style.rs`).
- Corners are square: the radius is 2px (`rounded-(--radius)`). Use checkboxes, not switches.
- Interface copy follows the [NSH Design Guides](https://nsh.dev/docs/design-guides.md):
  sentence case for labels, hints and states, with no final period on a short label. Vim's
  own text (`NORMAL`, `E37: ...`) stays as Vim writes it.

## Tests

```sh
TMPDIR=~/.cache/nsvim-test-tmp cargo test     # every test: core, app, Headless
cargo test -p editor-core                     # the core only, fast, no window
cargo clippy --all-targets -- -D warnings     # CI runs this
cargo fmt
```

Point `TMPDIR` at a folder outside `/tmp` and outside the checkout. Tests make project folders
in it.

The app tests in `ext/nsvim-native/tests/` start the real window in NSH's Headless engine, with no display.
They press keys and views, and they read texts, test ids and app state. For example:

```rust
let mut app = start(&file);
typ(&mut app, &["j", "d", "d"]);
app.wait_for("text:/let b/")?;
```

The helpers are in `ext/nsvim-native/tests/common/mod.rs`. They keep `HOME`, `XDG_CONFIG_HOME` and
`XDG_STATE_HOME` in a scratch folder, so no test writes to your home folder.

The bench tests print timings and check a budget. Run them in an optimized build:

```sh
TMPDIR=~/.cache/nsvim-test-tmp cargo test --profile quick -p nsvim-native \
  --test flows_bench -- --ignored --nocapture --test-threads=1
```

## Releases

Push a tag that starts with `v`:

```sh
git tag v0.4.0
git push origin v0.4.0
```

The tag must be the version in `nsh.json` and `ext/nsvim-native/Cargo.toml`, with a `v` in front.

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
