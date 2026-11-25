# How nsvim works

nsvim is two crates in one workspace. The **core** (`crates/editor-core`, library `editor`) is
the text and the rules for changing it. It knows nothing about windows. The **app** (`nsh/`,
binary `nsvim`) draws the core with [NSH](https://nsh.dev), routes input to it, and runs every
slow job on a worker thread. NSH draws with the native views of each platform: GTK 4 on Linux,
AppKit on macOS and WinUI on Windows.

```mermaid
flowchart LR
    subgraph UI["Engine thread"]
        K[Keys and mouse] --> R[Key routing<br/>nsh/src/app/keys.rs]
        R --> KM[Keymap<br/>vim or classic]
        KM --> E[(Editor<br/>buffers, cursor, mode)]
        KM -->|Action| A[App<br/>nsh/src/app]
        A --> E
        A --> S[Session<br/>nsh/src/store.rs]
        S -->|memos| V[Views<br/>nsh/src/ui]
        P[poll] --> A
    end
    subgraph W["Worker threads"]
        IO[Files<br/>open, save, swap, undo]
        IX[Project index]
        G[git]
        RG[ripgrep]
        FW[File watcher]
        L[LSP manager]
        C[Control socket<br/>ctl, mcp]
    end
    A -->|job| IO & IX & G & RG & L
    IO & IX & G & RG & FW & L & C -->|Event, then wake| P
```

## The core: `crates/editor-core`

| Module | Holds |
| --- | --- |
| `buffer` | `Buffer`: a [ropey](https://docs.rs/ropey) rope, the path, line endings, grouped undo, named marks that move with edits, and the undo history kept across restarts. |
| `editor` | `Editor`: open documents, the active one, the mode, registers, the command line, search, options, and the `Action`s a keymap returns. |
| `vim` | The Neovim keymap: Normal, Insert, Replace, Visual and Command modes, operators, motions, text objects, Ex commands, search and a small regex engine. |
| `classic` | The classic keymap: no modes, Shift to select, Ctrl or ⌘ shortcuts, and multiple cursors. |
| `syntax` | Tree-sitter text objects and motions, and syntax colours from each grammar's highlight query, for 13 languages. |

A keymap is a function from a key to changes:

```rust
pub fn handle_key(ed: &mut Editor, key: KeyInput) -> Vec<Action>
```

It changes the text and the cursor directly. Anything outside the text, such as saving a file,
opening the finder or copying to the system clipboard, comes back as an `Action` for the app to
carry out. This split keeps the core free of I/O. Its tests run in milliseconds and need no
window.

Every edit goes through `Buffer`, inside a transaction:

```rust
buf.begin(cursor);
buf.remove(a, b);
buf.insert_at(a, "new text");
buf.commit(cursor); // one undo step, with the cursor before and after
```

An Insert session, a `:s` command, a paste at every cursor or a rename across a file is one
undo step.

## The app: `nsh/`

| Path | Holds |
| --- | --- |
| `src/app` | `App`: the core `Editor`, the tree, overlays, Settings, setup, git and LSP state. Key routing, mouse handling, pickers and actions. Handlers record effects on a `Tx` (copy, read the clipboard, a menu, the title, quit, wake later). |
| `src/store.rs` | `Session`: `App` in a `stored` cell, a revision signal, and the effects runner. |
| `src/model.rs` | `frame`: what the text area shows, from the `Editor`. Plain Rust, with no NSH code. |
| `src/ui` | Views, one module for each area. Colours come only from theme tokens (`ui/style.rs`). |
| `src/services` | The worker threads, each with a job channel in and an `Event` channel out. |
| `src/ipc` | The control socket protocol, and the `ctl` and `mcp` commands. |
| `config.rs`, `theme.rs` | `config.toml`, the 16 built-in palettes, and the Omarchy theme reader. |

### One change

NSH runs a component body once. After that, only the bindings that read a changed signal run
again. The app uses this in three steps:

1. Every change goes through `Session::update(|app, tx| ...)`. It changes `App`, bumps the
   revision signal, and then carries out the effects the handler recorded.
2. Each view reads only its part of `App`, through a memo (`Session::view(|app| part)`). The
   memo runs again after each update. Its readers run again only when the part changed.
3. The text area has one slot for each visible row. Each slot reads its own row of the frame,
   so a key rebuilds only the rows that changed. The caret is a view on top of the rows.

Syntax colours come from tree-sitter highlight queries. They run only over the lines on screen.
Scrolling is the editor's own (`view.top`), not a scroll view, because the cursor and
`scrolloff` decide what shows.

### Input

One view takes every key (`capture_keys(KeyCapture::All)` in `src/lib.rs`) and gives it to
`App::key`. The key goes to the first of these that wants it:

1. First-launch setup.
2. Settings, while it shows. It covers the whole window.
3. An open overlay: the leader menu, a picker, the context menu.
4. The command line (`:`, `/`, `?`).
5. A hover or completion popup.
6. The file tree, when it has focus.
7. The active keymap.

The text area measures one character of the code font. The mouse code maps a point to a line
and a column from that width, and counts tabs and wide characters.

### Workers

Workers never touch app state. Each gets jobs on a channel and sends `Event`s back. Then it
calls a waker. The waker sends on a channel that a task on the engine thread awaits, and that
task polls the app. Events arrive while the window is hidden, covered or minimized, so an
agent's request or a file reload never waits for you to look.

| Worker | Does |
| --- | --- |
| `editor-io` | Opens files (with `.editorconfig`, a stored undo history and crash recovery), writes saves atomically, writes swap files and undo histories, reads previews. |
| `editor-index` | Walks the project with git's ignore rules, for the finder and the tree, and walks it again when files come or go. |
| `editor-git` | Reads the branch and file status, and diffs each buffer against `HEAD` for the gutter. |
| `editor-grep` | Runs `rg --vimgrep`, or a plain walk when ripgrep is missing. |
| `editor-watch` | Watches the project, open files outside it, `config.toml` and Omarchy's theme folder. |
| `editor-lsp` | Starts language servers, keeps documents in step, and turns answers into events. |
| `editor-probe` | Finds facts the setup needs: this program's path, `$PATH`, free command names, the Neovim leader. |
| `editor-ipc` | Serves the project's control socket for the `ctl` and `mcp` commands and agents. See [agents.md](agents.md). |

A save shows the pattern. The engine thread trims the text as one undo step and takes its
bytes. The file worker writes them to a temporary file and renames it over the old one. The
`Saved` event then marks that buffer version clean. The UI never waits on the disk.

## State on disk

| Path (Linux) | Holds |
| --- | --- |
| `~/.config/nsvim/config.toml` | Settings. |
| `~/.local/state/nsvim/sessions/` | Open files and cursor positions, per project. |
| `~/.local/state/nsvim/undo/` | Undo histories of saved files. |
| `~/.local/state/nsvim/swap/` | Unsaved text, for crash recovery. |
| `~/.local/state/nsvim/window.toml` | The window's last size and position. |

nsvim copies the files of an older `editor` folder into these folders once, on the first start.
