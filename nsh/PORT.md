# Porting editor from Rune to NSH: the brief for each part

Goal: the NSH app in `nsh/` does everything the Rune app in `../src` does, with automated tests
for each feature. The Rune app is the reference: its views are in `../src/ui`, and its state
code is already in `nsh/src/app` (ported, Rune removed).

## How the NSH app works

- `src/app/` is the editor state (`App`), from the Rune app with `rune::Tx` swapped for
  `app::fx::Tx`. Handlers record effects on it (copy, read the clipboard, menus, title, quit,
  wake later). `src/store.rs` carries them out. Input types (`Drag`, `WheelEvent`,
  `PointerEvent`, `Mods`) are in `app/fx.rs`.
- `src/store.rs`: `Session` (`Copy`) holds `App` in a `stored` cell. Change state only with
  `s.update(|app, tx| ...)` (or `update_with` for input with modifiers). Read state in a view
  only through a memo: `let part = s.view(|app| ...)`, where the part is `Clone + PartialEq`.
  Never read `App` in a component body and bind the value: bodies run once.
- `src/ui/`: one module per area. `ui/mod.rs` lays them out. Colours come only from
  `ui::style::color("token")`. The tokens and their roles are listed at the top of
  `ui/style.rs`. Do not write raw colours. A theme swatch that is data uses `style::swatch`.
- `src/model.rs`: the text area's frame, from the core `Editor`. Plain Rust.
- The core is `../crates/editor-core` (buffer, vim, classic, syntax and highlight).
- Workers (`src/services`) send `Event`s. `Session::start` polls the app when they wake it.

## NSH facts we learned (read before you write code)

- NSH is 0.7.0. Read `~/.claude/skills/nsh/SKILL.md`, then `references/rust.md`,
  `references/rust-ui.md`, `references/rust-testing.md`, and the parts of
  `references/coding-guides.md` you need. Look up every API in the crate source:
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/nsh-core-0.7.0`, `nsh-base-0.7.0`,
  `nsh-testing-0.7.0`. A full checkout of NSH 0.7.0 is at `/home/jason/work/nsh` (examples in
  `examples/`). Never invent an API.
- A component body runs once. Pass closures or memos to setters. Use `when(..)`, `each(..)`
  and `dynamic(..)` for structure that changes. `.child(..)` is decided once.
- `nsh_base` components have no look. Style every part, with tokens.
- In Headless tests a `window()` root shows only with `environment.os = Os::Linux`. The
  helpers in `tests/common/mod.rs` do this. Use them.
- The root view of a window hears no `on_layout`. Put layout listeners on inner views.
- On Linux, `focus()` works only once the window shows. The app retries (see `lib.rs`).
- The keys: one view in `lib.rs` takes every key (`capture_keys(KeyCapture::All)`) and sends
  them to `App::key`, which routes them (setup, overlays, command line, tree, keymap). Do not
  add other key handlers. Change the routing in `src/app` if a feature needs it.
- Headless does not draw. Assert texts, test ids (`id:..`), roles and labels. Give a view you
  test a `.test_id(..)`.

## Rules

- Follow `../AGENTS.md`: files under 800 lines, tests in sibling `*_tests.rs` or `tests/`,
  ASD-STE100 Simplified Technical English in comments and docs (short sentences, active voice,
  no em dashes, no semicolons in prose), no file I/O on the UI thread.
- Never use `/tmp`. Run tests with `TMPDIR=~/.cache/editor-test-tmp`. Put logs and scratch
  under `nsh/target/agents/<your part>/`.
- Do not open GUI windows (`cargo run`, `nsh run`): other agents and the user share the
  desktop. Prove your work with Headless tests and unit tests.
- Edit only the files your part owns (below). If you must change a shared file (`store.rs`,
  `lib.rs`, `ui/mod.rs`, `Cargo.toml`), keep the change small and additive, and say so in your
  report.
- Never add AI attribution to commits, code or docs. Commit on your branch with plain
  messages when your part is done.

## Done means

From `nsh/`, all pass with no warnings:

```sh
cargo fmt
TMPDIR=~/.cache/editor-test-tmp cargo test
cargo clippy --all-targets -- -D warnings
```

Each feature of your part has a test that fails if the feature breaks.

## Parts and the files they own

| Part | Owns |
| --- | --- |
| chrome | `../ui/buffer-line.tsx`, `../ui/tree.tsx`, `../ui/status-line.tsx`, `../ui/command-line.tsx`, `../ui/font.tsx`, `exports/buffer_line.rs`, `exports/tree.rs`, `exports/status.rs`, `exports/command.rs`, `ui/font.rs`, `app/tree.rs`, `tests/chrome.rs`, `tests/tree.rs`, `tests/status_line.rs` |
| text | `ui/text_area.rs`, `ui/popups.rs`, `ui/review_view.rs`, `model.rs`, `model_tests.rs`, `app/mouse.rs`, `app/review.rs`, `app/lsp.rs`, `tests/text.rs` |
| overlays | `ui/overlays.rs` (and new `ui/overlays_*.rs`), `app/picker.rs`, `app/key_list.rs`, `app/menu.rs`, `fuzzy.rs`, `tests/overlays.rs` |
| settings | `../ui/controls.tsx`, `../ui/settings/*.tsx`, `../ui/setup/*.tsx`, `exports/settings.rs`, `exports/setup.rs`, `app/settings.rs`, `app/setup.rs`, `tests/settings.rs` |
| runtime | `main.rs`, `store.rs`, `lib.rs`, `app/mod.rs`, `app/actions.rs`, `app/keys.rs`, `app/ipc.rs`, `app/session.rs`, `services/`, `ipc/`, `args.rs`, `config.rs`, `theme.rs`, `tests/runtime.rs` |
| flows | `tests/flows*.rs`: the Rune app's app tests (`port/tests.rs`, `port/live_tests.rs`, `port/bench_tests.rs`) as NSH tests |
