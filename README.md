<div align="center">

<img src="assets/editor.svg" width="96" alt="nsvim icon: an N in the Normal, Visual and Insert colors over an orange command line">

# nsvim

**A modal code editor with Vim keys, drawn with the native views of each platform.**

[Website](https://huacnlee.github.io/nsvim/) · [Install](#install) · [Keys](docs/keymaps.md) · [Configuration](docs/configuration.md) ·
[AI agents](docs/agents.md) · [How it works](docs/architecture.md)

</div>

## The name

nsvim has two meanings:

- **N**ext **S**imple **VIM**: a small, fast editor with the Vim keys you know.
- **N**ative **S**hell **VIM**: it draws its UI with [NSH (Native Shell)](https://nsh.dev).

## Where it comes from

nsvim is a fork of [editor](https://github.com/bjarneo/editor) by
[Bjarne Oeverli](https://github.com/bjarneo) and its contributors, under the MIT license. The
git history keeps their work, and [LICENSE](LICENSE) keeps their copyright.

nsvim is a separate project. It does not follow editor's releases, and its changes do not go
back to editor. Report nsvim issues here, not to editor.

The main change: nsvim draws its UI with [NSH](https://nsh.dev), which uses the native views of
each platform (GTK 4 on Linux, AppKit on macOS, WinUI on Windows). The NSH app is in
[`nsh/`](nsh). The editing core (buffer, Vim and classic keymaps, tree-sitter) is the crate
`editor-core` in [`crates/editor-core`](crates/editor-core).

## What it is

nsvim is for opening code, reading it, finding your way around it and making quick edits.

- **Real Vim, or classic keys.** Counts, operators, text objects, dot repeat, registers,
  macros, marks and Visual block. Or no modes at all, with multiple cursors. Pick one at first
  launch and switch any time.
- **Code-aware.** Tree-sitter text objects (`af`, `ic`, `ia`) and motions (`]f`, `[c`) in 13
  languages. LSP with diagnostics, go to definition, references, hover, completion, rename and
  code actions, from the servers you have installed.
- **Find anything.** A fuzzy file finder, project search through ripgrep, a file tree with git
  status, and a list of every key with search: `F1`.
- **Works with your AI agent.** No AI inside. Claude Code, Codex or any CLI agent sees what you
  look at and drives nsvim over `nsvim mcp` or `nsvim ctl`. It reads your selection and unsaved
  text, points you at code, and proposes changes that you accept with `y`.
- **Safe.** Crash recovery, undo that survives a restart, changed lines marked when a git
  checkout or an agent changes a file, and atomic saves.
- **Yours.** 16 themes. On [Omarchy](https://omarchy.org), the System look follows the desktop
  theme live. Any monospace font. Settings is a buffer that writes `config.toml`.

## Install

```sh
cargo install --locked --git https://github.com/huacnlee/nsvim nsh-app-editor
```

This installs the program `nsvim`. On Linux, the build needs the GTK 4 development packages.

## Quick start

```sh
nsvim .                 # this folder as the project
nsvim src/main.rs:42    # a file, at line 42
```

The first launch runs a five-step setup: keys, mouse, look, terminal and ready. The terminal
step makes `vi` and `vim` open nsvim (on by default, `\vim` still runs Vim), adds the command
`nsvim`, sets `$EDITOR` and sets git's editor. On Linux, it can also make nsvim the default app
for text files. It shows every command before it runs, and it runs only the ones you leave on.
After setup, `vim .` opens nsvim. Turn the aliases off to keep Vim, and use `nsvim .`.

## Keys

| Action | Neovim | Classic |
| --- | --- | --- |
| Save | `:w` | `Ctrl S` |
| Go to file | `SPC f` | `Ctrl P` |
| Find in project | `SPC /` | `Ctrl ⇧ F` |
| Command line | `:` | |
| Command palette | | `Ctrl ⇧ P` |
| Every key, with search | `SPC ?`, `F1` | `F1` |
| Leader keys, in a popup | `SPC` | |
| Go to definition | `gd` | `F12` |
| Rename symbol | `SPC l n` | `F2` |
| Select a function | `vaf` | |
| Next function | `]f` | |
| Add a cursor at the next match | | `Ctrl D` |
| Focus the file tree | `SPC e` | `Ctrl ⇧ E` |
| Settings | `:Settings` | `Ctrl ,` |

On macOS, `⌘` replaces `Ctrl` in the classic keymap. [docs/keymaps.md](docs/keymaps.md) has
every key.

## Configuration

Settings is a tab. Open it with `:Settings` or `SPC ,` in the Neovim keymap, or with `Ctrl ,`
in the classic keymap. `:w` writes `~/.config/editor/config.toml`. Every change applies at
once. [docs/configuration.md](docs/configuration.md) lists every option.

## AI agents

```sh
claude mcp add nsvim -- nsvim mcp     # Claude Code: nsvim as tools
nsvim ctl state                       # any agent or script: what you look at
nsvim ctl reveal src/main.rs:10-14 --note "this is the bug"
```

[docs/agents.md](docs/agents.md) has the setup for Claude Code, Codex and opencode.

## Development

```sh
cd nsh
cargo run -- path/to/file.rs
TMPDIR=~/.cache/editor-test-tmp cargo test
cargo clippy --all-targets -- -D warnings
```

Read [AGENTS.md](AGENTS.md) and [nsh/README.md](nsh/README.md) before you change the code.

## License

nsvim is released under the MIT license. See [LICENSE](LICENSE).

nsvim is based on [editor](https://github.com/bjarneo/editor), the original work of
[Bjarne Oeverli](https://github.com/bjarneo) and its contributors, also under the MIT license.
Their copyright notice stays in [LICENSE](LICENSE), as the license asks. The editing core, the
keymaps, the tree-sitter support, the agent socket and much of the design come from their work.
Thank you to them.

- Copyright (c) 2026 The editor authors: the original work.
- Copyright (c) 2026 Jason Lee and the nsvim contributors: the changes in nsvim.

If you copy or change nsvim, keep both copyright lines and the license text.
