# Command line

You type `nvim`. It is a shell alias that the first-run setup makes, with `vim` and `vi`. Each
one runs the NsVim program by its full path. `\nvim` still runs Neovim. The installer also
puts the command `nsvim` on `PATH` (`~/.local/bin` on macOS and Linux, the install folder on
Windows).

Scripts and AI agents do not see aliases. They run `nsvim`, or the program by its full path. This page
calls it PROGRAM. `nvim --help` prints it. The installer puts it at `~/.local/share/nsvim/nsvim`
on Linux, and at `~/Applications/NsVim.app/Contents/MacOS/nsvim` on macOS.

The program opens files and folders. `ctl` drives an NsVim that is already running, from a
shell, a script or an AI agent. `mcp` serves that same NsVim to agents that speak the Model
Context Protocol.

```sh
nvim .                           # open this folder as the project
nvim src/main.rs:42              # open a file at line 42
nvim ctl state                   # what the running editor shows, as JSON
claude mcp add nsvim -- ~/.local/share/nsvim/nsvim mcp
```

## Open files and folders

```text
nvim [PATH[:LINE]]... [+LINE] [--wait] [--new] [--tui]
```

| Form | Does |
| --- | --- |
| `nvim` | Opens the current folder as the project, with the files of the last session. |
| `nvim .` | The same, for the folder you name. |
| `nvim src/main.rs` | Opens the file, in the project around it. |
| `nvim src/main.rs:42` | Opens the file with the cursor on line 42. |
| `nvim +42 src/main.rs` | The same. `+LINE` applies to the first file. |
| `nvim a.rs b.rs` | Opens both files. The last one is active. |
| `nvim . src/lib.rs` | Opens the folder as the project, and the file in it. Paths are relative to the current folder. |
| `nvim --wait NOTES.md` | Returns when you close that buffer. See [Use NsVim as `$EDITOR`](#use-nsvim-as-editor). |
| `nvim --new` | Opens a new window, also when NsVim runs for the project already. |
| `nvim --tui src/main.rs` | Edits in the terminal instead of a window. See [The terminal editor](#the-terminal-editor). |
| `nvim --help`, `nvim -h` | Prints the usage. |
| `nvim --version`, `nvim -V` | Prints the version: `nsvim 0.2.0`. |

A file that does not exist opens as a new, empty buffer. `:w` creates it.

### The project folder

The project is the folder the file tree, the finder and project search use. NsVim picks it
this way:

1. A folder you name is the project.
2. For a file, the top folder of its git repository is the project.
3. Without git, the nearest folder above the file with one of these files is the project:
   `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `CMakeLists.txt`,
   `Makefile` or `.editorconfig`. The search stops at your home folder.
4. Else the file's own folder is the project.

### One window per project

This applies to windows, not to `--tui`. When NsVim already runs for the project, `nvim PATH` opens the file in that window,
brings the window forward, and exits at once. On Wayland, the new process passes its
`XDG_ACTIVATION_TOKEN`, so the window takes focus. `nvim` with no file only brings the
window forward. `--new` skips this and always opens a new window.

### The terminal editor

NsVim opens a window, from a launcher (Finder, the Start menu, a desktop entry) and from a
terminal alike. `--tui` edits in the terminal instead, as vim does: over SSH, in tmux, or
anywhere you type `nvim --tui`. The old `-g` flag is still accepted and changes nothing. It is the same editor. It runs the same screens and the same Rust core, so
the keymaps, the finder, search, the file tree, git signs, the language servers, settings and
the session all work as in the window. On Linux, NsVim also starts in the terminal when there
is no display (`DISPLAY` and `WAYLAND_DISPLAY` are not set).

Every release has both editors in one package. On Windows, `nsvim.exe` is a window program
and gets no console, so the terminal editor is the second program in the package:

```text
nsvim-tui.exe [PATH[:LINE]]...
```

The terminal editor always starts a new editor. It does not open files in a running window.
The terminal draws text only, so icons show as chars and the font settings do not apply.

### Exit status

| Status | Means |
| --- | --- |
| `0` | The window closed, the file opened in the running editor, or `--wait` returned. |
| `1` | The running editor did not answer, or could not open the file. The reason is on stderr. |

## Use NsVim as `$EDITOR`

`--wait` makes NsVim a blocking editor, for git, `crontab -e` and agents that open a prompt
in your editor:

```sh
export EDITOR="PROGRAM --wait" VISUAL="PROGRAM --wait"
git config --global core.editor "PROGRAM --wait"
```

Git and other tools do not see aliases, so these lines name PROGRAM, the full path.

With NsVim running for the project, the file opens there, and the command returns when
you close that buffer: `:q`, `:wq`, `ZZ`, `SPC q` or `Ctrl W`. Without one, NsVim opens a
window and returns when you close the window. Setup's terminal step can add these lines for
you.

## `ctl`

```text
PROGRAM ctl [--socket PATH] [--json] COMMAND [ARGS]
```

In your own terminal, `nvim ctl` is the same command. `ctl` sends one request to the running editor of the current folder and prints the
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
nvim ctl reveal src/main.rs:10-14 --note "this is the bug"
sed 's/foo/bar/' src/main.rs | nvim ctl propose src/main.rs --label "rename foo"
nvim ctl read --lines 1-20
nvim ctl state | jq '.active | {relative, line, dirty}'
```

### Which editor answers

`ctl` and `mcp` find the editor in this order:

1. The socket in `$EDITOR_SOCKET`, when an editor answers there.
2. The editor whose project is the current folder, or a folder above it.
3. The only running editor, when exactly one runs.

Each editor serves one socket for its project, at `$XDG_RUNTIME_DIR/nsvim/<hash>.sock`.
Without `XDG_RUNTIME_DIR`, the socket is in a folder of your own under the temp folder. Only
your user can read it, and there is no network listener. `ipc.enabled = false` turns the
socket off.

### Exit status

| Status | Means |
| --- | --- |
| `0` | Done. |
| `1` | The editor refused the request. The reason is on stderr, such as a file that is not open, or `edit` when `ipc.allow_edits` is off. |
| `2` | No editor runs for this folder, the editor did not answer in 10 seconds, or the command is wrong. |

## `mcp`

`PROGRAM mcp` is a Model Context Protocol server over stdio, for the editor running in the
current folder. An agent starts it. Add it once:

```sh
claude mcp add nsvim -- ~/.local/share/nsvim/nsvim mcp       # Claude Code
```

The server gives the agent 12 tools: `editor_state`, `editor_selection`, `editor_read`,
`editor_diagnostics`, `editor_changes`, `editor_open`, `editor_reveal`, `editor_message`,
`editor_propose`, `editor_edit`, `editor_save` and `editor_command`. Each one is the `ctl`
command of the same name. Setup for Codex and opencode, and what each tool does, is in
[agents.md](agents.md).

## Environment

| Variable | Does |
| --- | --- |
| `EDITOR_SOCKET` | The socket `ctl` and `mcp` use first. |
| `XDG_RUNTIME_DIR` | The folder for the control sockets. |
| `XDG_CONFIG_HOME`, `XDG_STATE_HOME` | Move the config and state folders on Linux. |
| `XDG_ACTIVATION_TOKEN` | Lets a second `nvim` command give the running window focus on Wayland. |
| `SHELL` | Setup's terminal step writes to `~/.zshrc` for zsh, else `~/.bashrc`. |
| `EDITOR_TIMING` | `EDITOR_TIMING=1 nvim .` prints how long each step of startup takes. |

## Files

| Linux | macOS | Holds |
| --- | --- | --- |
| `~/.config/nsvim/config.toml` | `~/Library/Application Support/nsvim/config.toml` | Settings. See [configuration.md](configuration.md). |
| `~/.local/state/nsvim/sessions/` | `~/Library/Application Support/nsvim/sessions/` | Open files and cursor positions, per project. |
| `~/.local/state/nsvim/undo/` | `~/Library/Application Support/nsvim/undo/` | Undo histories of saved files. |
| `~/.local/state/nsvim/swap/` | `~/Library/Application Support/nsvim/swap/` | Unsaved text, for crash recovery. |
| `~/.local/state/nsvim/window.toml` | `~/Library/Application Support/nsvim/window.toml` | The window's last size and place. |
