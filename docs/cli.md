# Command line

`editor` opens files and folders. `nsvim ctl` drives an editor that is already running, from
a shell, a script or an AI agent. `nsvim mcp` serves that same editor to agents that speak the
Model Context Protocol.

```sh
nsvim .                          # open this folder as the project
nsvim src/main.rs:42             # open a file at line 42
nsvim ctl state                  # what the running editor shows, as JSON
claude mcp add nsvim -- nsvim mcp
```

## Open files and folders

```text
nsvim [PATH[:LINE]]... [+LINE] [--wait] [--new]
```

| Form | Does |
| --- | --- |
| `editor` | Opens the current folder as the project, with the files of the last session. |
| `nsvim .` | The same, for the folder you name. |
| `nsvim src/main.rs` | Opens the file, in the project around it. |
| `nsvim src/main.rs:42` | Opens the file with the cursor on line 42. |
| `nsvim +42 src/main.rs` | The same. `+LINE` applies to the first file. |
| `editor a.rs b.rs` | Opens both files. The last one is active. |
| `nsvim . src/lib.rs` | Opens the folder as the project, and the file in it. Paths are relative to the current folder. |
| `nsvim --wait NOTES.md` | Returns when you close that buffer. See [Use editor as `$EDITOR`](#use-editor-as-editor). |
| `nsvim --new` | Opens a new window, also when an editor runs for the project already. |
| `editor --help`, `editor -h` | Prints the usage. |
| `editor --version`, `editor -V` | Prints the version: `editor 0.3.2`. |

A file that does not exist opens as a new, empty buffer. `:w` creates it.

### The project folder

The project is the folder the file tree, the finder and project search use. editor picks it
this way:

1. A folder you name is the project.
2. For a file, the top folder of its git repository is the project.
3. Without git, the nearest folder above the file with one of these files is the project:
   `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `CMakeLists.txt`,
   `Makefile` or `.editorconfig`. The search stops at your home folder.
4. Else the file's own folder is the project.

### One window per project

When an editor already runs for the project, `editor PATH` opens the file in that window,
brings the window forward, and exits at once. On Wayland, the new process passes its
`XDG_ACTIVATION_TOKEN`, so the window takes focus. `editor` with no file only brings the
window forward. `--new` skips this and always opens a new window.

### Exit status

| Status | Means |
| --- | --- |
| `0` | The window closed, the file opened in the running editor, or `--wait` returned. |
| `1` | The running editor did not answer, or could not open the file. The reason is on stderr. |

## Use editor as `$EDITOR`

`--wait` makes editor a blocking editor, for git, `crontab -e` and agents that open a prompt
in your editor:

```sh
export EDITOR="nsvim --wait" VISUAL="nsvim --wait"
git config --global core.editor "nsvim --wait"
```

With an editor running for the project, the file opens there, and the command returns when
you close that buffer: `:q`, `:wq`, `ZZ`, `SPC q` or `Ctrl W`. Without one, editor opens a
window and returns when you close the window. Setup's terminal step can add these lines for
you.

## `nsvim ctl`

```text
nsvim ctl [--socket PATH] [--json] COMMAND [ARGS]
```

`nsvim ctl` sends one request to the running editor of the current folder and prints the
reply. Lines and columns are 1-based. Paths are absolute or relative to the project folder.

| Flag | Does |
| --- | --- |
| `--json` | Prints every reply as JSON, also for `read` and `selection`. |
| `--socket PATH` | Talks to the editor at this socket. See [Which editor answers](#which-editor-answers). |
| `-h`, `--help` | Prints the commands. |

### Read

| Command | Prints |
| --- | --- |
| `state` | JSON: the project folder, mode, keymap, the active file, every open buffer, diagnostic counts, outside changes and proposals. |
| `selection` | The selected text. Nothing when there is no selection. |
| `read [PATH] [--lines A-B]` | A buffer's text, unsaved edits included. Without `PATH`, the active buffer. |
| `diagnostics [PATH]` | JSON: the language server's errors and warnings, for one file or every open file. |
| `changes` | JSON: blocks changed on disk while the editor was open, newest first. |
| `list` | Every running editor: its project folder and its socket, one per line. |

`state` returns this shape:

```json
{
  "root": "/home/you/code/app",
  "mode": "NORMAL",
  "keymap": "neovim",
  "active": {
    "path": "/home/you/code/app/src/main.rs",
    "relative": "src/main.rs",
    "language": "rust",
    "line": 42,
    "column": 5,
    "lines": 310,
    "dirty": false,
    "selection": null,
    "visible": { "first": 20, "last": 63 }
  },
  "buffers": [{ "path": "…", "relative": "src/main.rs", "dirty": false, "lines": 310, "active": true }],
  "diagnostics": { "errors": 0, "warnings": 2 },
  "outside_changes": 0,
  "proposals": 0,
  "settings_open": false
}
```

When text is selected, `selection` holds `{"start": {"line", "column"}, "end": {…}, "text"}`.

### Act

| Command | Does |
| --- | --- |
| `open PATH[:LINE[:COL]] [--wait]` | Opens a file and brings the window forward. `--wait` returns when its buffer closes. |
| `reveal PATH:LINE[-END] [--note TEXT]` | Jumps to the lines, highlights them and shows the note beside the first one. |
| `reveal --clear` | Removes the highlight and the note. `Esc` in the editor does the same. |
| `message TEXT [--error]` | Shows a line on the command line. `--error` shows it in the error colour. |
| `propose PATH [--label TEXT]` | Reads the new text of the file from stdin and shows it as a diff. You press `y` to accept or `n` to reject. |
| `edit PATH [--label TEXT]` | Reads the new text from stdin and puts it in the buffer at once, as one undo step. |
| `save [PATH] [--all]` | Writes a buffer, or every modified buffer with `--all`. |
| `command ':EX'` | Runs an Ex command, such as `':w'` or `':%s/old/new/g'`. |

`edit` and `command` change your buffers without a review. `ipc.allow_edits = false` in
`config.toml` turns both off, and leaves `propose`. `edit` and `propose` open a file that is
not open yet. A file that does not exist starts empty. Nothing is written until you save.

```sh
nsvim ctl reveal src/main.rs:10-14 --note "this is the bug"
sed 's/foo/bar/' src/main.rs | nsvim ctl propose src/main.rs --label "rename foo"
nsvim ctl read --lines 1-20
nsvim ctl state | jq '.active | {relative, line, dirty}'
```

### Which editor answers

`nsvim ctl` and `nsvim mcp` find the editor in this order:

1. The socket in `$EDITOR_SOCKET`, when an editor answers there.
2. The editor whose project is the current folder, or a folder above it.
3. The only running editor, when exactly one runs.

Each editor serves one socket for its project, at `$XDG_RUNTIME_DIR/editor/<hash>.sock`.
Without `XDG_RUNTIME_DIR`, the socket is in a folder of your own under the temp folder. Only
your user can read it, and there is no network listener. `ipc.enabled = false` turns the
socket off.

### Exit status

| Status | Means |
| --- | --- |
| `0` | Done. |
| `1` | The editor refused the request. The reason is on stderr, such as a file that is not open, or `edit` when `ipc.allow_edits` is off. |
| `2` | No editor runs for this folder, the editor did not answer in 10 seconds, or the command is wrong. |

## `nsvim mcp`

`nsvim mcp` is a Model Context Protocol server over stdio, for the editor running in the
current folder. An agent starts it. Add it once:

```sh
claude mcp add nsvim -- nsvim mcp       # Claude Code
```

The server gives the agent 12 tools: `editor_state`, `editor_selection`, `editor_read`,
`editor_diagnostics`, `editor_changes`, `editor_open`, `editor_reveal`, `editor_message`,
`editor_propose`, `editor_edit`, `editor_save` and `editor_command`. Each one is the `editor
ctl` command of the same name. Setup for Codex and opencode, and what each tool does, is in
[agents.md](agents.md).

## Environment

| Variable | Does |
| --- | --- |
| `EDITOR_SOCKET` | The socket `nsvim ctl` and `nsvim mcp` use first. |
| `XDG_RUNTIME_DIR` | The folder for the control sockets. |
| `XDG_CONFIG_HOME`, `XDG_STATE_HOME` | Move the config and state folders on Linux. |
| `XDG_ACTIVATION_TOKEN` | Lets a second `editor` command give the running window focus on Wayland. |
| `SHELL` | Setup's terminal step writes to `~/.zshrc` for zsh, else `~/.bashrc`. |
| `EDITOR_TIMING` | `EDITOR_TIMING=1 nsvim .` prints how long each step of startup takes. |

## Files

| Linux | macOS | Holds |
| --- | --- | --- |
| `~/.config/nsvim/config.toml` | `~/Library/Application Support/nsvim/config.toml` | Settings. See [configuration.md](configuration.md). |
| `~/.local/state/nsvim/sessions/` | `~/Library/Application Support/nsvim/sessions/` | Open files and cursor positions, per project. |
| `~/.local/state/nsvim/undo/` | `~/Library/Application Support/nsvim/undo/` | Undo histories of saved files. |
| `~/.local/state/nsvim/swap/` | `~/Library/Application Support/nsvim/swap/` | Unsaved text, for crash recovery. |
| `~/.local/state/nsvim/window.toml` | `~/Library/Application Support/nsvim/window.toml` | The window's last size and place. |
