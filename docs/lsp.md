# Language servers

nsvim uses any language server that is installed and on your `PATH`. There is nothing to
configure: open a file, and its server starts.

| Language | Server |
| --- | --- |
| Rust | `rust-analyzer` |
| Go | `gopls` |
| Python | `pyright-langserver --stdio` |
| TypeScript, JavaScript, TSX, JSX | `typescript-language-server --stdio` |
| C, C++ | `clangd` |
| Lua | `lua-language-server` |
| Zig | `zls` |

rustup installs a `rust-analyzer` proxy that fails until the component exists:

```sh
rustup component add rust-analyzer
```

When a server starts or stops, the command line says so. A server that exits at once says
"exited before it started": run it from a terminal to see why.

## Features

| Feature | Neovim | Classic |
| --- | --- | --- |
| Diagnostics in the gutter, under the code and in the status line | | |
| A list of this file's diagnostics | `SPC d` | |
| Go to definition | `gd`, `SPC l d` | `F12` |
| References, in a picker | `SPC l r` | |
| Hover | `K`, `SPC l h` | |
| Completion | while you type | while you type |
| Rename | `SPC l n`, `:Rename name` | `F2` |
| Code actions, in a picker | `SPC l a`, `:CodeAction` | `Ctrl .` |

Completion opens at the second char of a word, and after `.` and `::`. It filters as you type.

Rename opens the command line with `:Rename` and the word under the cursor. Edit the name and
press `Enter`. Each open file changes as one undo step. Files that are not open load in the
background with the changes made and stay unsaved: `:wa` writes them all.

## How it runs

One thread owns every server. It speaks JSON-RPC to each over stdin and stdout, and a reader
thread per server forwards what the server sends. The UI thread only sends jobs and reads
events, so a slow or stuck server never blocks typing.

- **Sync.** The app sends a file's full text 150 ms after you stop typing, and at once before a
  request, so answers match what you see.
- **Positions.** nsvim asks servers to count columns in chars (UTF-32). For servers that count
  UTF-16 units, it converts with the line's text.
- **Large files.** Files over 4 MiB do not go to a server.
- **Server requests.** nsvim answers `workspace/configuration` and applies
  `workspace/applyEdit`, which code actions often send.
