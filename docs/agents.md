# AI agents and scripts

NSVim has no AI inside it. Instead, a running NSVim is something an agent in your terminal
can see and drive: Claude Code, Codex, opencode, or a shell script. The agent can read what you
are looking at, point you at code, propose changes you review as a diff, and use the editor's
language server diagnostics.

## Connect an agent

Every running NSVim serves a socket for its project. Two commands of the NSVim program use it:

- `mcp`, a [Model Context Protocol](https://modelcontextprotocol.io) server over stdio. Agents
  that speak MCP get NSVim as tools.
- `ctl`, a command-line client for any agent or script.

Agents and scripts do not see the shell aliases that setup makes (`nvim`, `vim`, `vi`). They
run the program by its full path. `nvim --help` prints it. The examples use the Linux path of
the installer, `~/.local/share/nsvim/nsvim`. On macOS, it is
`~/Applications/nsvim.app/Contents/MacOS/nsvim`. Do not give an agent `nvim`: outside your
interactive shell, `nvim` is Neovim.

### Claude Code

```sh
claude mcp add nsvim -- ~/.local/share/nsvim/nsvim mcp
```

### Codex

```toml
# ~/.codex/config.toml
[mcp_servers.nsvim]
command = "/home/you/.local/share/nsvim/nsvim"
args = ["mcp"]
```

### opencode

```json
{
  "mcp": {
    "nsvim": { "type": "local", "command": ["/home/you/.local/share/nsvim/nsvim", "mcp"] }
  }
}
```

### Any other agent

Tell it about `ctl` in its instructions file (`AGENTS.md`, `CLAUDE.md`). Put in the full path
of your program:

```markdown
The user reads code in nsvim. Run `~/.local/share/nsvim/nsvim ctl` from the project folder.
Do not run `nvim`: it is Neovim here.
- `~/.local/share/nsvim/nsvim ctl state` shows the open file, the cursor and the selection.
- `~/.local/share/nsvim/nsvim ctl read PATH` returns a buffer's text with unsaved edits.
- `~/.local/share/nsvim/nsvim ctl diagnostics` returns the language server's errors.
- `~/.local/share/nsvim/nsvim ctl reveal PATH:LINE-END --note TEXT` points the user at code.
- `cat new.rs | ~/.local/share/nsvim/nsvim ctl propose PATH --label TEXT` shows a change for the
  user to accept.
```

## What an agent can do

| Tool (`mcp`) | Command (`ctl`) | Does |
| --- | --- | --- |
| `editor_state` | `state` | The active file, cursor, selection, visible lines, open buffers and unsaved state |
| `editor_selection` | `selection` | The selected text |
| `editor_read` | `read [PATH] [--lines A-B]` | A buffer's text, unsaved edits included |
| `editor_diagnostics` | `diagnostics [PATH]` | The language server's errors and warnings |
| `editor_changes` | `changes` | Changes made on disk while the editor was open, newest first |
| `editor_open` | `open PATH[:LINE[:COL]]` | Opens a file and brings the window forward |
| `editor_reveal` | `reveal PATH:LINE[-END] --note TEXT` | Jumps to lines and highlights them, with a note |
| `editor_message` | `message TEXT [--error]` | Shows a line on the command line |
| `editor_propose` | `propose PATH [--label TEXT] < new.txt` | Shows the new text as a diff. The user presses `y` to accept or `n` to reject |
| `editor_edit` | `edit PATH [--label TEXT] < new.txt` | Changes the buffer at once, as one undo step. A file that is not open, or does not exist, opens unsaved |
| `editor_save` | `save [PATH] [--all]` | Writes a buffer, or every modified one |
| `editor_command` | `command ':EX'` | Runs an Ex command, such as `:w` or `:%s/a/b/g` |

[cli.md](cli.md) has every command, flag and exit status. Lines and columns are 1-based. Paths are
absolute or relative to the project root. `ctl`
prints JSON for data and plain text for `read` and `selection`; add `--json` for JSON
everywhere. It exits 0 when done, 1 when the editor refuses, and 2 when no NSVim is running.

In your own terminal, the alias works, so `nvim ctl` is the same command:

```sh
$ nvim ctl state | jq '.active | {relative, line, dirty}'
{ "relative": "src/app/keys.rs", "line": 42, "dirty": false }

$ nvim ctl reveal src/app/keys.rs:40-48 --note "this match decides who gets the key"
$ sed 's/println!/eprintln!/' src/main.rs | nvim ctl propose src/main.rs --label "log to stderr"
```

## Review what changed

- **Proposals.** A proposal opens as a diff in the editor: removed lines in red, added lines in
  green, unchanged runs folded. `y` accepts it as one undo step, `n` rejects it, `]c` and `[c`
  move between changes, and `j`, `k`, `Ctrl d` and `Ctrl u` scroll. The status line counts
  waiting proposals.
- **Outside changes.** When an agent, git or any tool writes a file you have open, the editor
  reloads it and marks the changed lines with `┃` in the gutter. `SPC a l` (or the palette's
  "Agent: list outside changes") lists every changed block; `u` undoes the reload. Saving the
  file, or `SPC a c`, clears the marks. A buffer with unsaved edits is never reloaded over: the
  editor warns instead.
- **Notes.** `reveal` tints the lines and shows the note beside the first one. `Esc` clears it.

## `$EDITOR`, git and second launches

`nvim PATH` opens the file in the NSVim already running for that project, brings it forward
and exits. `nvim --wait PATH` waits until you close that buffer, so it works as `$EDITOR` for
git commit messages and for agents that open a prompt in your editor. `--new` always opens a new
window.

## Shell filters

`:'<,'>!cmd` runs the selected lines through any command and replaces them with its output, as
one undo step. `:!cmd` runs a command in the project folder and shows its last line. Both run
off the UI thread. This works with any CLI, AI tools included:

```vim
:'<,'>!sort -u
:'<,'>!jq .
:'<,'>!llm "rewrite this as a table"
```

## Safety

- The socket is `$XDG_RUNTIME_DIR/nsvim/<project>.sock`, readable by your user only. There is
  no network listener.
- `ipc.allow_edits = false` in config.toml (Settings › keys & mouse) turns off `edit` and
  `command`. Proposals still work: nothing changes until you press `y`.
- `ipc.enabled = false` turns the socket off.
- Every agent action shows on the command line.

## Protocol

To write your own client: connect to the socket and send one JSON object per line. Each line
gets one reply.

```json
{"id": 1, "cmd": "reveal", "args": {"path": "src/main.rs", "line": 10, "end_line": 12, "note": "here"}}
{"id": 1, "ok": true, "result": {}}
```

A failed request answers `{"id": 1, "ok": false, "error": "..."}`. `open` with `"wait": true`
answers only when the buffer closes. Requests run even while the window is hidden or on another
workspace.
