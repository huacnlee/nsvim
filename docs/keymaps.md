# Keys

NsVim has two keymaps. Setup asks which one you want. To switch later, run
`:set keymap=classic` or `:set keymap=neovim`, use Settings › keys & mouse, or pick
`Keymap: Neovim` or `Keymap: classic` in the command palette.

This file shows each key as you press it. `SPC` is the leader. `Ctrl d` is Control
and `d` together. `⇧` is Shift. `Enter`, `Esc`, `Tab`, `Backspace`, `Delete`, `Home`, `End`,
`PageUp`, `PageDown` and the arrows are those keys. `{N}` is a count that you type first, and
`{c}` is any char.

## Neovim keymap

Space is the leader by default. To use backslash, as in stock Neovim, set `keys.leader` to
`backslash`. Setup can also read the leader from `~/.config/nvim`. In Visual mode and after an
operator, `Space` moves right, as `l` does. With the backslash leader, it moves right in Normal
mode too.

### Leader menu

The leader menu and the [command palette](#command-palette) run the same commands: each
palette command has leader keys, and each leader key runs a palette command. The palette
finds a command by name, the leader menu by keys.

Press the leader alone to see these keys in a popup. Press a key from the popup, or click
it. `Esc` or any other key closes the popup. With the Space leader, the status line shows
`Space for keys`. A click on it opens the popup too.

| Keys | Does |
| --- | --- |
| `SPC f` | Find a file |
| `SPC o` | Open a project: a recent one, or a folder from the system's dialog. See [Open a project](#open-a-project) |
| `SPC O` | Open a folder from the system's dialog |
| `SPC /` | Search the project with ripgrep |
| `SPC b` | Pick an open buffer |
| `SPC e` | Show and focus the file tree. From the tree, hide it |
| `SPC w` | Write the buffer |
| `SPC W` | Write every modified buffer |
| `SPC q` | Close the buffer. A modified buffer stays open |
| `SPC Q` | Quit. A modified buffer stops it |
| `SPC d` | List the diagnostics of this file |
| `SPC ,` | Open Settings |
| `SPC ?` | Show every key, with search. See [Key list](#key-list) |
| `SPC p` | Open the command palette. See [Command palette](#command-palette) |
| `SPC g n`, `SPC g p` | Next, previous git change against `HEAD`. Wraps around |
| `SPC g s` | Show and focus the file tree, with its git status letters |
| `SPC g b` | Show or hide who last changed each line |
| `SPC l d` | Go to the definition |
| `SPC l r` | List the references |
| `SPC l h` | Show hover information |
| `SPC l n` | Rename the symbol. Opens `:Rename` with the word under the cursor |
| `SPC l a` | List the code actions |
| `SPC a l` | List the changes made on disk from outside the editor |
| `SPC a c` | Clear the outside-change marks |
| `SPC a p` | Show the next agent proposal |
| `SPC a n` | Clear the agent's note |
| `SPC t n`, `SPC t p` | Next, previous tab |
| `SPC t o` | Close the other tabs |
| `SPC t l`, `SPC t r` | Close the tabs to the left, to the right of this tab |
| `SPC t c` | Close the tabs with no unsaved changes |
| `SPC t a` | Close all tabs |
| `SPC s v`, `SPC s s` | Split the pane: move this tab to a new pane on the right, below. See [Panes](#panes) |
| `SPC s n`, `SPC s p` | Give the keys to the next, the previous pane |
| `SPC s c` | Close the pane. Its tabs join the pane next to it |
| `SPC y u`, `SPC y r` | Undo, redo one change |
| `SPC y x`, `SPC y y`, `SPC y p` | Cut, copy, paste, on the Visual selection |
| `SPC y a` | Select every line |
| `SPC y f`, `SPC y s` | Open the find bar, with its replace field for `SPC y s`. See [Find and replace](#find-and-replace) |
| `SPC u w` | Turn soft wrap on or off |
| `SPC u v`, `SPC u c` | Switch to the Neovim keymap, to the classic keymap |
| `SPC u d`, `SPC u l`, `SPC u s` | Dark theme, light theme, follow the OS |
| `SPC u S` | Open App Setup to change keymap, theme and other picks. See [Setup](#setup) |

The `SPC l` keys need a language server. See `docs/lsp.md`.

### Motions

Every motion takes a count. Each one also works after an operator, such as `d` or `y`, and
in Visual mode.

| Keys | Moves to |
| --- | --- |
| `h` `j` `k` `l`, arrows | Left, down, up, right |
| `Backspace`, `Ctrl h` | Left |
| `Ctrl n` `Ctrl j`, `Ctrl p` | Down, up |
| `gj` `gk` | Down, up. Same as `j` `k`, also with soft wrap on |
| `+` `Enter` `Ctrl m`, `-` | First non-blank of the next line, the line above |
| `_` | First non-blank of this line. `{N}_` goes `N-1` lines down |
| `w` `b` `e` `ge` | Next word start, word start before, word end, word end before |
| `W` `B` `E` `gE` | The same for WORDs, which only blanks separate |
| `0` `Home` `g0` | Line start |
| `^` | First non-blank |
| `$` `End` `g$` | Line end. `{N}$` goes to the end of the line `N-1` lines down |
| `g_` | Last non-blank |
| `{N}\|` | Column `N` |
| `gg`, `G` | First line, last line. `{N}gg` and `{N}G` go to line `N` |
| `{N}%` | The line `N` percent into the file |
| `f{c}` `F{c}` | Next, previous `{c}` on the line |
| `t{c}` `T{c}` | Just before the next `{c}`, just after the previous `{c}` |
| `;` `,` | Repeat the last `f` `F` `t` `T`, the same way, the other way |
| `%` | The bracket that matches the first `(` `[` `{` `)` `]` `}` at or after the cursor on the line |
| `{` `}` | Previous, next empty line |
| `(` `)` | Previous, next sentence start |
| `H` `M` `L` | Top, middle, bottom line of the screen. `{N}H` and `{N}L` count from the edge |
| `n` `N` | Next, previous match of the last search |
| `*` `#` | Next, previous match of the whole word under the cursor |
| `g*` `g#` | The same, also inside longer words |
| `]f` `[f` | Next, previous function start |
| `]F` `[F` | Next, previous function end |
| `]c` `[c` | Next, previous class start |
| `]C` `[C` | Next, previous class end |
| `]a` `[a` | Next, previous parameter or argument start |
| `]A` `[A` | Next, previous parameter or argument end |
| `'{mark}` | First non-blank of the mark's line |
| `` `{mark} `` | The mark's exact place |

The `]` and `[` motions use tree-sitter. They work in Rust, Python, JavaScript, TypeScript,
TSX, Go, C, C++, Java, Ruby, Bash, JSON and Lua files. A language has only the kinds it
defines. Bash, JSON and Lua have no classes. JSON has no functions and no parameters, and
Bash has no parameters.

### Scroll keys and the jump list

| Keys | Does |
| --- | --- |
| `Ctrl d` `Ctrl u` | Scroll half a screen down, up. `{N}Ctrl d` scrolls `N` lines |
| `Ctrl f` `PageDown`, `Ctrl b` `PageUp` | Scroll a screen down, up |
| `Ctrl e` `Ctrl y` | Scroll one line down, up. The cursor stays on its line while the line is on the screen |
| `zz` `z.` | Put the cursor's line in the middle of the screen |
| `zt` `z Enter` | Put the cursor's line at the top |
| `zb` `z-` | Put the cursor's line at the bottom |
| `{N}zz` `{N}zt` `{N}zb` | Go to line `N`, then put it in the middle, at the top, at the bottom |
| `Ctrl o` | Go back in the jump list, across files |
| `Ctrl i` `Tab` | Go forward in the jump list |
| `''` ``` `` ``` | Go to the place before the last jump |

The jump list keeps the place before each jump, up to 100 places. `G`, `gg`, `%`, `{`, `}`,
`(`, `)`, `H`, `M` and `L` are jumps. So are `n`, `N`, `*`, `#`, `g*`, `g#` and the `]` and `[`
motions. A search, a mark and `:{N}` are jumps too.

### Operators and text objects

| Operator | Does |
| --- | --- |
| `d` | Delete |
| `c` | Change: delete, then Insert mode |
| `y` | Yank |
| `>` `<` | Indent, outdent |
| `=` | Reindent: the indent of the line above, one level more after `{` `(` `[`, one level less before `}` `)` `]` |
| `gu` `gU` `g~` | Lower case, upper case, toggle case |
| `gc` | Toggle line comments. The comment marker comes from the file type |

An operator takes a motion or a text object: `dw`, `c$`, `y}`, `ci(`. Double an operator for
whole lines: `dd`, `cc`, `yy`, `>>`, `<<`, `==`, `gcc`, `guu`, `gUU`, `g~~`. `gugu`, `gUgU` and
`gcgc` work too. An operator also takes a search: `d/pat` and then `Enter` deletes up to the next
match, and `c?pat` and then `Enter` changes back to the previous one. `dgn` and `cgn` act on
the next search match.

| Object | Around | Inner |
| --- | --- | --- |
| Word, WORD | `aw` `aW` | `iw` `iW` |
| Sentence | `as` | `is` |
| Paragraph | `ap` | `ip` |
| Parentheses | `a(` `a)` `ab` | `i(` `i)` `ib` |
| Brackets | `a[` `a]` | `i[` `i]` |
| Braces | `a{` `a}` `aB` | `i{` `i}` `iB` |
| Angle brackets | `a<` `a>` | `i<` `i>` |
| Quotes | ``a" a' a` `` | ``i" i' i` `` |
| HTML or XML tag | `at` | `it` |
| Function, tree-sitter | `af` | `if` |
| Class, struct, enum, trait or impl, tree-sitter | `ac` | `ic` |
| Parameter or argument, tree-sitter | `aa` | `ia` |
| Comment, tree-sitter | `a/` | `i/` |

Brackets and tags work across lines. Quotes work on the cursor's line. A count picks the
object around the one you are in: `d2af` deletes the function around this one, and `2dit`
deletes the tag around this tag. When there is no tag or tree-sitter object at the cursor, the
status line says why.

### Other Normal mode keys

| Keys | Does |
| --- | --- |
| `i` `a` | Insert before, after the cursor |
| `I` `A` | Insert at the first non-blank, at the line end |
| `o` `O` | Open a line below, above |
| `gi` | Insert where Insert mode ended last |
| `R` | Replace mode. See the section below |
| `x` `Delete`, `X` | Delete the char under the cursor, the char before it |
| `s`, `S` | Change the char, the line |
| `D`, `C` | Delete, change to the line end |
| `Y` | Yank to the line end, as in Neovim |
| `p` `P` | Put after, before the cursor |
| `J` `gJ` | Join lines with a space, without a space |
| `r{c}` | Replace the char under the cursor. `r Enter` splits the line |
| `~` | Toggle the case of the char and move right |
| `u`, `Ctrl r` | Undo, redo |
| `U` | Put the last changed line back as it was. `U` again brings the changes back |
| `.` | Repeat the last change. A Visual change repeats on a region of the same size |
| `Ctrl a` `Ctrl x` | Add one to, subtract one from the number at or after the cursor |
| `&` | Run the last `:s` again on this line, without its flags |
| `v` `V` `Ctrl v` `Ctrl q` | Visual mode: char, line, block |
| `gv` | Select the last Visual selection again |
| `gn` `gN` | Select the next, previous search match |
| `:` | Open the command line. `{N}:` fills in the range `.,.+N-1` |
| `/` `?` | Search forward, backward |
| `gd` `gD` | Go to the definition, with the language server |
| `K` | Show hover information, with the language server |
| `gf` | Open the file whose name is under the cursor. It looks beside this file, then at the project root |
| `gt` `gT` | Next, previous tab |
| `Ctrl w h`, `Ctrl w w`, `Ctrl w Ctrl w` | Show the file tree and give it the focus. `Ctrl w p` does the same |
| `Ctrl w v`, `Ctrl w s` | Split the pane: move this tab to a new pane on the right, below. See [Panes](#panes) |
| `Ctrl w l`, `Ctrl w j` | Give the keys to the next pane. With one pane, nothing changes |
| `Ctrl w k` | Give the keys to the previous pane |
| `Ctrl w c` | Close the pane. Its tabs join the pane next to it |
| `ZZ` | Write the buffer if it changed, then close it. After the last buffer, quit |
| `ZQ` | Close the buffer. Do not write it |
| `Esc` `Ctrl c` `Ctrl [` | Cancel a command you started to type |
| `F2` | Rename the symbol. Opens `:Rename` with the word under the cursor |

The status line shows the keys of a command that is not complete, such as `"a2d`.

### Insert mode

| Keys | Does |
| --- | --- |
| `Esc` `Ctrl [` `Ctrl c` | Go back to Normal mode |
| `Ctrl o` | Run one Normal mode command, then come back to Insert mode |
| `Ctrl r {reg}` | Insert register `a` to `z`, `0` or `"` |
| `Ctrl w` | Delete the word before the cursor |
| `Ctrl u` | Delete to the first non-blank, then to the line start |
| `Backspace` `Ctrl h` | Delete the char before the cursor |
| `Delete` | Delete the char under the cursor |
| `Enter` `Ctrl j` `Ctrl m` | Split the line |
| `Tab` | Type spaces to the next tab stop. With `expandtab` off, type a tab char |
| `Ctrl t` `Ctrl d` | Indent, outdent the line |
| Arrows, `Home`, `End`, `PageUp`, `PageDown` | Move |

When `keys.auto_close` is on, `(`, `[`, `{` and the quotes also type their pair. If you type
`)`, `]`, `}` or a quote where the same char is already, the cursor steps over it. `Backspace`
between a pair deletes both chars. In the indent at the line start, `Backspace` goes back one
indent level.

`Enter` keeps the line's indent. After `{`, `(` or `[` it adds one level, and between `{}` it
puts the `}` on its own line. In Python files, `Enter` after `:` adds one level too. A count repeats the insert: `3ix` and then `Esc` types `xxx`.

The completion list shows while you type. See [Completion list](#completion-list).

### Replace mode

`R` starts Replace mode. The status line says `REPLACE`.

- Each char you type takes the place of the char under the cursor. Past the line end, it adds
  a char.
- `Backspace` puts the old char back. Before the first replaced char, it only moves left.
- `Enter` splits the line. `Tab` types one indent level over the text.
- Arrows and `Ctrl` keys work as in Insert mode. After them, `Backspace` cannot put back the
  chars that came before.
- `Esc` ends Replace mode. A count repeats the text: `3Rab` and then `Esc` types `ababab`.
  `.` repeats it.

### Visual mode

`v`, `V` and `Ctrl v` start char, line and block selection. `Ctrl q` is the same as `Ctrl v`.
Press the same key again to stop, or another one of them to change the kind of selection.
`Esc`, `Ctrl c` and `Ctrl [` stop too.

Every motion extends the selection, and so does a search with `/` or `?`. A text object such
as `iw` or `af` selects the object. An object of whole lines, such as `ip`, changes the
selection to lines.

| Keys | Does |
| --- | --- |
| `o` `O` | Go to the other end of the selection |
| `d` `x` | Delete |
| `y` | Yank |
| `c` `s` | Change |
| `>` `<` | Indent, outdent. `3>` indents three levels |
| `=` | Reindent |
| `~` `g~`, `u` `gu`, `U` `gU` | Toggle case, lower case, upper case |
| `gc` | Toggle comments on the lines |
| `D` `X` | Delete the lines. In a block, delete to each line end |
| `Y` | Yank the lines |
| `C` `S` `R` | Change the lines. `C` in a block changes to each line end |
| `J` `gJ` | Join the lines with a space, without a space |
| `r{c}` | Replace every selected char with `{c}` |
| `p` `P` | Replace the selection with the clipboard or a register |
| `I` `A` | Insert before, after the selection. In a block, type on every line |
| `$A` | In a block, append at the end of every line |
| `:` | Open the command line with the range `'<,'>` |
| `"{reg}` | Use a register for the next command: `"ay` |

`.` repeats a Visual change on a region of the same size at the cursor. `gv` selects the last
selection again. The marks `'<` and `'>` hold its start and end.

### Registers

| Register | Holds |
| --- | --- |
| `"`, `+`, `*` | The unnamed register. Every yank and delete goes there and to the system clipboard. `p` and `P` put the system clipboard |
| `a` to `z` | Named registers. `"ayy` yanks the line into `a`, and `"ap` puts it |
| `A` to `Z` | Append to `a` to `z`: `"Ayy` adds the line to `a` |
| `0` | The last yank. A delete does not change it |
| `1` to `9` | Deletes and changes of a line or more, newest in `1`. A new one moves the others down |
| `-` | The last delete inside one line, such as `x` or `dw` |
| `.` | The text typed in the last Insert session |
| `:` | The last Ex command. `@:` runs it again |
| `/` | The last search pattern |
| `_` | The black hole. `"_dd` deletes and keeps every register as it was |

A register name comes before the command: `"ayiw`, `"bp`, `"_x`. In Insert mode, `Ctrl r a`
inserts register `a`. Ex commands take a register name after the command: `:d a`, `:y b`,
`:put c`.

`Ctrl r +` and `Ctrl r *` insert the system clipboard, also text that another app copied.
`Ctrl r "` inserts the last yank or delete made in the editor.

### Marks

| Keys | Does |
| --- | --- |
| `m{a-z}` | Set a mark in this buffer. It moves with the edits above it |
| `m{A-Z}` | Set a file mark. `'A` opens its file from any buffer |
| `'{mark}` | Go to the first non-blank of the mark's line |
| `` `{mark} `` | Go to the mark's exact place |
| `d'a` ``y`a`` | An operator up to a mark |
| `:marks` | List the marks |
| `:delmarks a b`, `:delmarks c-e`, `:delmarks!` | Delete marks, a span of marks, every mark from `a` to `z` |

The editor also sets these marks:

| Mark | Is |
| --- | --- |
| `'` | The place before the last jump. `''` goes there. `m'` sets it |
| `.` | The last change |
| `^` | Where Insert mode ended last. `gi` goes there |
| `<` `>` | The start and end of the last Visual selection |

An operator cannot reach a file mark in another file. It shows `E20: Mark not set`.

### Macros

| Keys | Does |
| --- | --- |
| `q{a-z}` | Record keys into register `{a-z}`. The status line says `recording @a` |
| `q{A-Z}` | Record keys and append them to register `{a-z}` |
| `q` | End the macro |
| `@{a-z}` | Play a register. `{N}@a` plays it `N` times |
| `@@` | Play the last register again |
| `@:` | Run the last Ex command again |

Any register plays as keys, so `"ayy` and then `@a` runs the yanked line as keys.

### Counts

A count is a number before the command. It does not start with `0`.

| Example | Does |
| --- | --- |
| `3j`, `3w`, `3f,` | Move three times |
| `3dw`, `d3w`, `2d3w` | Delete three words. Two counts multiply: `2d3w` deletes six |
| `3dd`, `3yy`, `3>>`, `3gcc` | Act on three lines |
| `5G`, `5gg`, `50%`, `8\|` | Go to line 5, the middle of the file, column 8 |
| `3p`, `3x`, `3~`, `3r-`, `3J` | Put three times, delete three chars, toggle three chars, replace three chars, join three lines |
| `3ix` then `Esc`, `2o`, `3R` | Repeat the inserted text |
| `3.` | Repeat the last change with the count 3: after `dd`, `3.` deletes three lines |
| `3u`, `3Ctrl r`, `3@a` | Undo, redo or play three times |
| `5Ctrl a`, `5Ctrl x` | Add or subtract five |
| `2n`, `2*` | Go to the second match |
| `d2aw`, `d2af`, `2dit` | Act on the second object around the cursor |
| `3:` | Open the command line with `.,.+2` |
| `10zt` | Go to line 10 and put it at the top of the screen |
| `5Ctrl d`, `3Ctrl e`, `2Ctrl f` | Scroll five lines, three lines, two screens |

### Search

`/pat` and then `Enter` searches forward. `?pat` and then `Enter` searches backward. An empty
pattern searches for the last one again. The search wraps around the file, and the status line
says when it does. `:noh` hides the match highlights until the next search.

The pattern takes Vim's magic syntax. It knows `.` `*` `\+` `\=` `\?` `\{n,m}` `\{-n,m}` `^` `$`
`[abc]` `[^abc]` `\(group\)` `\%(group\)` `a\|b` `\<` `\>`. It knows the classes `\s` `\S`
`\d` `\D` `\w` `\W` `\a` `\l` `\u` `\x` `\h`. A pattern with an uppercase letter matches case. A
pattern without one ignores case. `\c` makes a pattern ignore case, and `\C` makes it match
case. A match never crosses a line end.

### Command line

These keys work while you type a `:`, `/` or `?` line.

| Keys | Does |
| --- | --- |
| `Enter` `Ctrl m` `Ctrl j` | Run the line |
| `Esc` `Ctrl c` `Ctrl [` | Close the line |
| `Backspace` `Ctrl h` | Delete the char before the caret. On an empty line, close it |
| `Delete` | Delete the char under the caret |
| `Ctrl w` | Delete the word before the caret |
| `Ctrl u` | Delete everything before the caret |
| `Left` `Right` | Move the caret |
| `Home` `Ctrl b`, `End` `Ctrl e` | Go to the start, the end |
| `Up` `Ctrl p`, `Down` `Ctrl n` | Older, newer history that starts with the text you typed |
| `Tab` | Type a tab char. There is no completion |

`:` lines and search lines have separate histories of 200 lines each.

### Ex commands

A command takes a range when the table shows one. The next section explains ranges.

| Command | Does |
| --- | --- |
| `:w`, `:write` | Write the buffer |
| `:w path`, `:sav path`, `:saveas path` | Write to `path`. The buffer takes the new path |
| `:wa`, `:wall` | Write every modified buffer |
| `:q`, `:quit`, `:clo`, `:close` | Close the buffer. After the last buffer, quit |
| `:q!` | Close the buffer and drop its changes |
| `:wq`, `:wq path` | Write the buffer, then close it |
| `:x`, `:xit`, `:exi`, `:exit` | Write the buffer if it changed, then close it |
| `:qa`, `:qall`, `:quita`, `:quitall` | Quit. A modified buffer stops it |
| `:qa!` | Quit and drop every change |
| `:wqa`, `:wqall`, `:xa`, `:xall` | Write every modified buffer, then quit when the writes are done |
| `:e path`, `:edit path` | Open a file. A relative path starts at the project root. `~/` is your home folder |
| `:enew`, `:ene` | Open a new empty buffer with no name. `:w path` names and writes it; `:w` alone says E32. Save in the menu or the palette asks where to save it |
| `:e` | Read the buffer again from disk. With unsaved changes, it stops with `E37` |
| `:e!` | Read the buffer again from disk and drop its changes |
| `:vs`, `:vsp`, `:vsplit` | Move this tab to a new pane on the right. `:vsplit path` opens the file first |
| `:sp`, `:split` | Move this tab to a new pane below. `:split path` opens the file first |
| `:bn`, `:bnext` | Next tab |
| `:bp`, `:bprev`, `:bprevious`, `:bN`, `:bNext` | Previous tab |
| `:bd`, `:bdelete`, `:bw`, `:bwipeout` | Close the buffer. A modified buffer stays open |
| `:ls`, `:buffers`, `:files`, `:b`, `:buffer` | Pick an open buffer |
| `:b N`, `:b name` | Go to buffer `N`, counted from 1, or to the one buffer whose path holds `name` |
| `:Find`, `:find`, `:Files` | Find a file |
| `:Projects`, `:Project`, `:Open` | Open a project |
| `:Open path`, `:cd path` | Open the folder `path` as the project. A terminal has no folder dialog, so type the folder here |
| `:NewFile path`, `:Create path`, `:Touch path` | Make an empty file and open it. A `/` at the end makes a folder. The folders above it are made too |
| `:Mkdir path`, `:NewFolder path` | Make a folder and the folders above it |
| `:Move path` | Move or rename the tree's row when the tree has the focus, else the open file. Onto a folder, it goes inside. Open buffers follow |
| `:Delete`, `:Remove` | Delete the tree's row when the tree has the focus, else the open file, and close its buffers. `!` deletes unsaved changes too |
| `:Grep`, `:grep`, `:rg` | Search the project |
| `:Explore`, `:Ex`, `:Tree` | Show or hide the file tree |
| `:Settings`, `:settings` | Open Settings |
| `:Setup` | Open App Setup, from the config in use. See [Setup](#setup) |
| `:Keys [words]`, `:help [words]`, `:h [words]` | Show every key. The words go in the search line |
| `:Commands` | Open the command palette |
| `:Rename name`, `:rename name` | Rename the symbol under the cursor, with the language server |
| `:CodeAction`, `:codeaction`, `:ca` | List the code actions, with the language server |
| `:{range}` | Go to the last line of the range: `:42`, `:$`, `:'a` |
| `:{range}s/pat/rep/flags`, `:substitute` | Substitute. See [Substitute](#substitute) |
| `:{range}&`, `:{range}&&` | Run the last `:s` again, without its flags, with its flags |
| `:{range}g/pat/cmd`, `:global` | Run the Ex command `cmd` on every line that matches. The range is the whole file if you give none |
| `:{range}g!/pat/cmd`, `:{range}v/pat/cmd`, `:vglobal` | Run `cmd` on every line that does not match |
| `:{range}d [x] [N]`, `:de`, `:del`, `:delete` | Delete the lines, into register `x`. With `N`, delete `N` lines from the range's last line |
| `:{range}y [x] [N]`, `:ya`, `:yank` | Yank the lines, into register `x` |
| `:{line}pu [x]`, `:{line}put [x]` | Put register `x` as lines below the line. `:put!` puts them above |
| `:{range}m {addr}`, `:mo`, `:move` | Move the lines below line `addr`. `:m0` moves them to the top |
| `:{range}t {addr}`, `:co`, `:copy` | Copy the lines below line `addr`. `:t0` copies them to the top |
| `:{range}j`, `:join` | Join the lines with spaces. With one line, join it and the next one. `:j!` joins without spaces |
| `:{range}>`, `:{range}<` | Indent, outdent the lines. `:>>` indents two levels |
| `:{range}sort`, `:sor` | Sort the lines. The range is the whole file if you give none |
| `:sort!`, `:sort i`, `:sort n`, `:sort u` | Sort in reverse order, without case, by the first number on the line, with no duplicate lines |
| `:{range}normal keys`, `:norm` | Type `keys` in Normal mode on each line of the range |
| `:u`, `:undo`, `:red`, `:redo` | Undo, redo |
| `:marks`, `:delmarks`, `:delm` | List, delete marks |
| `:noh`, `:nohlsearch` | Hide the search highlights until the next search |
| `:set ...`, `:se`, `:setl`, `:setlocal` | Set options for every buffer. See [Options](#options) |
| `:!cmd` | Run a shell command in the project folder and show its output |
| `:{range}!cmd` | Send the lines through a shell command and put its output in their place |

`:g` and `:v` take any Ex command: `:g/TODO/d`, `:v/\S/d`, `:g/^/m0`, `:g/fn /normal A;`.
With no command, `:g` counts the lines that match.

### Ranges

A range is one address, or two addresses with `,` or `;` between them. With `;`, the cursor
goes to the first address before the second is read: `:10;+2d` deletes lines 10 to 12.

| Address | Is |
| --- | --- |
| `%` | Every line |
| `.` | The cursor's line |
| `$` | The last line |
| `12` | Line 12 |
| `'a`, `'<`, `'>` | The line of a mark. Visual mode fills in `'<,'>` |
| `/pat/`, `?pat?` | The next, previous line that matches. The search wraps around the file |
| `+N`, `-N` | `N` lines after, before the cursor's line. `+` and `-` alone are one line. `.+3`, `$-1` and `'a+2` add to an address |

`{N}:` in Normal mode fills in a range: `5:` opens `:.,.+4`. A line command with no range acts
on the cursor's line. `:g`, `:v` and `:sort` with no range act on every line.

### Substitute

`:s/pat/rep/` replaces the first match on each line of the range. `:%s/pat/rep/g` replaces
every match in the file. Any char that is not a letter, a digit, `\`, `"` or `|` can take the
place of `/`: `:s#a/b#c#`. An empty pattern uses the last search.

| Flag | Does |
| --- | --- |
| `g` | Replace every match on a line, not only the first |
| `c` | Ask before each change |
| `n` | Count the matches. Change nothing |
| `e` | Show no error when nothing matches |
| `i`, `I` | Ignore case, match case |
| A number | Act on that many lines from the range's last line: `:s/a/b/ 3` |

In the replacement, `&` and `\0` are the whole match and `\1` to `\9` are the groups. `\n` and
`\r` split the line and `\t` is a tab. `\u` and `\l` change the case of the next char. `\U`
and `\L` change the case of every char after them, until `\e` or `\E`. `\&` is a plain `&`.

With the `c` flag, the editor selects each match and asks. These keys answer:

| Keys | Does |
| --- | --- |
| `y` | Replace this match, then go to the next one |
| `n` | Skip this match |
| `a` | Replace this match and every match after it |
| `l` | Replace this match, then stop |
| `q` `Esc` `Ctrl c` `Ctrl [` | Stop |
| `Ctrl e` `Ctrl y` | Scroll down, up |

All the changes of one `:s` undo as one step.

### Options

`:set name` turns an option on, `:set noname` turns it off, `:set name!` flips it and
`:set name=value` gives it a value. `:set name?` shows the value. One `:set` takes several
options: `:set ts=2 noet wrap`. A change applies at once and
shows in Settings. To keep it in `config.toml`, change it in Settings.

| Option | Short | Values |
| --- | --- | --- |
| `keymap` | | `neovim`, `classic` |
| `theme` | | `light`, `dark`, `system`, or a theme id from `docs/configuration.md`, such as `gruvbox` |
| `number` | `nu` | `:set nu` shows absolute numbers, `:set nonu` hides them. `:set nu=hybrid`, `relative`, `absolute` or `off` sets the style |
| `numbers` | | `hybrid`, `relative`, `absolute`, `off` |
| `relativenumber` | `rnu` | `:set rnu` shows hybrid numbers, `:set nornu` shows absolute numbers |
| `cursorline` | `cul` | On, off |
| `list` | | On, off. On shows the indent spaces as dots |
| `colorcolumn` | `cc`, `ruler` | A column for the guide line, `0` for none |
| `wrap` | | On, off |
| `expandtab` | `et` | On, off. On makes `Tab` type spaces |
| `tabstop` | `ts` | The indent width, a number |
| `shiftwidth`, `softtabstop` | `sw`, `sts` | The same as `tabstop` |
| `ligatures` | | On, off. Font ligatures such as `->` and `!=` |
| `hlsearch` | `hls` | On, off. On highlights every match of the last search |
| `ignorecase` | `ic` | On, off. On makes search ignore case. On by default |
| `smartcase` | `scs` | On, off. With `ignorecase`, an uppercase letter in the pattern makes case count. On by default |
| `scrolloff` | `so` | Lines kept above and below the cursor |
| `mouse` | | `:set mouse` and `:set mouse=a` turn the mouse on, `:set nomouse` and `:set mouse=` turn it off |

`\c` in a pattern ignores case and `\C` makes it count, whatever the options say.
`hlsearch`, `ignorecase` and `smartcase` last until the editor closes. An `.editorconfig` file sets the indent of its files, and it takes precedence over `tabstop` and
`expandtab`.

## Classic keymap

There are no modes: you type at once. On macOS, `Ctrl` is `⌘` and `Alt` is `⌥` in the
classic keymap.

| Keys | Does |
| --- | --- |
| `Ctrl S` | Save. A new file with no name asks where to save it |
| `Ctrl N` | New file, with no name yet |
| `Ctrl P` | Go to file |
| `Ctrl O` | Open a project |
| `Ctrl ⇧ P` | Command palette |
| `Ctrl F` | Find in the file |
| `Ctrl Alt F` | Replace in the file |
| `Ctrl ⇧ F` | Find in the project |
| `Ctrl Z` | Undo |
| `Ctrl ⇧ Z`, `Ctrl Y` | Redo |
| `Ctrl C` `Ctrl X` `Ctrl V` | Copy, cut, paste. With no selection, copy and cut take the whole line |
| `Ctrl A` | Select all |
| `Ctrl ⇧ K` | Delete the line, or every line of the selection |
| `Alt ↑` `Alt ↓` | Move the line, or the lines of the selection, up, down |
| `Ctrl /` | Toggle comments |
| `Ctrl ]` `Ctrl [` | Indent, outdent |
| `Tab` | Indent a selection of several lines. Else type one indent level |
| `⇧ Tab` | Outdent the line, or every line of the selection |
| `F12` | Go to the definition |
| `F2` | Rename the symbol. Opens `:Rename` with the word under the cursor |
| `Ctrl .` | List the code actions |
| `Ctrl B` | Show or hide the file tree |
| `Ctrl \` | Move this tab to a new pane on the right. `⌘ \` on macOS. See [Panes](#panes) |
| `Ctrl ⇧ E` | Show the file tree and give it the focus. From the tree, give the focus back to the text |
| `Ctrl Tab` `Ctrl ⇧ Tab` | Next, previous tab |
| `Ctrl Q` | Quit. A modified buffer stops it |
| `Ctrl W` | Close the tab. A modified tab stays open |
| `Ctrl ,` | Settings |
| `F1` | Show every key, with search |
| `Esc` | Drop the selection |

The text that you type between two cursor moves undoes as one step. A copied whole line
pastes above the cursor's line. A paste or a typed char over a selection takes its place.
Brackets, quotes, `Enter` and `Backspace` work as in the Neovim keymap's Insert mode.

### Move and select

| Keys | Does |
| --- | --- |
| Arrows | Move. With a selection, `←` and `→` go to its start and end |
| `Home` | Go to the first non-blank. Again, go to the line start |
| `End` | Go to the line end |
| `PageUp` `PageDown` | Move a screen up, down |
| `Ctrl Home` `Ctrl End` | Go to the start, the end of the file |
| `Ctrl ←` `Ctrl →` | Go a word left, right. On macOS, `Alt ←` and `Alt →` |
| `Ctrl Backspace` `Ctrl Delete` | Delete the word before, after the cursor. On macOS, `Alt Backspace` and `Alt Delete` |
| `⌘ ←` `⌘ →`, `⌘ ↑` `⌘ ↓` | On macOS only: line start and end, file start and end |
| `⇧` and any move above | Select from the cursor to where the move goes |

### Find and replace

`Ctrl F` opens the find bar over the text. `Ctrl Alt F` opens it with its replace field.
The selection on one line, else the word under the cursor, is the query. Type, and the bar
highlights every match, selects the first one from the cursor and shows `3 of 12`. Edit ›
Find… and Replace… and the palette's `Find in file…` and `Replace in file…` open the same
bar, in both keymaps. The terminal editor opens the search line and `:%s/` in its place.

These keys work while the bar has the keys. A click on the text gives the keys to the text,
and the bar stays open.

| Keys | Does |
| --- | --- |
| `Enter`, `⇧ Enter` | In the find field: go to the next, the previous match. Both wrap around the file |
| `Ctrl G`, `Ctrl ⇧ G`, `F3`, `⇧ F3` | Go to the next, the previous match |
| `Enter` | In the replace field: replace the selected match, then go to the next one |
| `Ctrl Enter` | In the replace field: replace every match in the file, as one undo step |
| `Tab` | Move between the find and the replace field |
| `Alt C`, `Alt W`, `Alt R` | Match case, whole word, regular expression. The bar's check boxes do the same |
| `Ctrl V` | Paste into the field |
| `Ctrl U`, `Ctrl Backspace` | Clear the field |
| `Esc` | Close the bar and its highlight. The keys go back to the text |

Without Match case, case does not matter. A regular expression takes the common syntax:
`(a|b)+`, `\d{2}`, `[a-z]*?`, `(?:x)`. In its replacement, `$1` is a group, `$&` the match
and `$$` a dollar. Plain text matches as typed.

The search line stays as the Vim search: `/`, `?`, `n` and `N` in the Neovim keymap, with the
syntax that [Search](#search) shows. `:%s/old/new/g` replaces with it, and with the `c` flag,
`%s/old/new/gc` asks at each match, with the keys that [Substitute](#substitute) lists.

### Multiple cursors

| Keys | Does |
| --- | --- |
| `Ctrl D` | Select the word under the cursor. Again, add a cursor at the next match of the selection |
| `Ctrl ⇧ L` | Put a cursor on every match of the selection, or of the word under the cursor |
| `Ctrl Alt ↑` `Ctrl Alt ↓` | Add a cursor above the top cursor, below the bottom cursor |
| `Alt`-click | Add a cursor, or remove the one under the pointer. `keys.multi_cursor` turns it on and off |
| `Esc` | Go back to one cursor |

These keys act at every cursor, as one undo step: typed chars, `Enter`, `Backspace`, `Delete`
and `Tab`. The arrows, `Home`, `End` and `⇧` with a move act at every cursor too. So do
`Ctrl C`, `Ctrl X`, `Ctrl V`, `Ctrl /` and `Ctrl ⇧ K`. A copy joins the selections, one per line. A paste with one line per cursor
puts one line at each. Cursors that meet become one. `Ctrl D`, `Ctrl S`, `Ctrl P`, `Ctrl B`,
`Ctrl W` and `Ctrl ,` keep the cursors. Other keys go back to one cursor first.

## Everywhere

These keys work in both keymaps, unless a row says otherwise.

### File tree

`SPC e`, `SPC g s` and `Ctrl w h` in the Neovim keymap, and `Ctrl ⇧ E` in the classic keymap,
show the tree and give it the focus. `Ctrl w w` and `Ctrl ⇧ E` move the focus between the tree
and the text. `:Tree` and the classic `Ctrl B` show or hide it. A click on a folder gives
the tree the focus. A click on a file or on the text gives the focus back. The mouse wheel over the tree scrolls it, with or without the
focus. These keys work while the tree has the focus.

| Keys | Does |
| --- | --- |
| `j` `↓`, `k` `↑` | Move down, up |
| `Ctrl d`, `Ctrl u` | Move half a screen down, up. Neovim keymap |
| `PageDown`, `PageUp` | Move a screen down, up |
| `g` `Home`, `G` `End` | Go to the first, the last row |
| `l` `→` | Open a closed folder, or open a file |
| `Enter` `o` | Open or close a folder, or open a file |
| `h` `←` | Close an open folder, or go to the parent folder |
| `a` | Make a file. Opens `:NewFile` with the row's folder. End the name with `/` to make a folder |
| `A` | Make a folder. Opens `:Mkdir` with the row's folder |
| `r` `m` `F2` | Rename or move the row. Opens `:Move` with its path. Edit the path and press `Enter` |
| `d` `Delete` | Delete the row, a folder with all in it. Press `y` to delete, any other key to keep it |
| `x`, `p` | Cut the row, then move it into the folder of the row under `p` |
| `C` `O` | Open the folder row as the project |
| `Esc` `q` | Give the focus back to the text |
| `Ctrl w l`, `Ctrl w w`, `Ctrl w Ctrl w` | Give the focus back to the text. `Ctrl w p` does the same. `Ctrl w h` keeps the focus in the tree. Neovim keymap |
| `Ctrl ⇧ E` | Give the focus back to the text. Classic keymap |
| `?` | Show every key, with search |
| `:` | Open the command line. Neovim keymap |
| `SPC` | Open the leader menu. Neovim keymap |
| `Ctrl` shortcuts | Work as in the text: `Ctrl P` finds a file, `Ctrl S` writes |

Other keys do nothing while the tree has the focus, so a key meant for the tree never edits
the file. A letter after a file name is its git status. A dot after a folder name shows that a file in
it changed.

### Panes

The open files show in panes, each with its own tabs. A tab dragged to an edge of a pane
splits it. These commands do the same from the keys, the menus and the palette:

| Command | Neovim keymap | Classic keymap | Does |
| --- | --- | --- | --- |
| Split right | `Ctrl w v`, `:vsplit`, `SPC s v` | `Ctrl \` (`⌘ \`) | Move this tab to a new pane on the right |
| Split down | `Ctrl w s`, `:split`, `SPC s s` | | Move this tab to a new pane below |
| Focus next pane | `Ctrl w l`, `Ctrl w j`, `SPC s n` | | Give the keys to the next pane |
| Focus previous pane | `Ctrl w k`, `SPC s p` | | Give the keys to the previous pane |
| Close pane | `Ctrl w c`, `SPC s c` | | Remove the pane. Its tabs join the pane before it, or after it for the first pane. No file closes |

The pane of the file that has the keys is the active pane. A file shows in one pane only, so
a split moves the tab. A pane with one tab has nothing to split: the command line says so.
The next pane is the next one from left to right, then from top to bottom. After the last
pane comes the first.

`Ctrl w h`, `Ctrl w w`, `Ctrl w Ctrl w` and `Ctrl w p` still go to the file tree, so they do not
move between panes. In Vim, `:close` closes the window. In NsVim it closes the tab, as before.

The View menu and the tab menu (a right-click on a tab) have the same commands.

### Pickers

The file finder, project search, the buffer list, the command palette, the key list,
references, diagnostics, outside changes and code actions open in one picker.

| Keys | Does |
| --- | --- |
| Chars | Filter the list. In project search, search again |
| `↑` `Ctrl p` `Ctrl k`, `↓` `Ctrl n` `Ctrl j` | Move up, down |
| `Tab`, `⇧ Tab` | Move down, up |
| `PageUp` `PageDown` | Move ten rows |
| `Backspace` | Delete the last char of the query |
| `Ctrl w` | Delete the last word of the query |
| `Ctrl u` | Clear the query |
| `Enter` | Open or run the selected row |
| `Esc` | Close the picker |

References, diagnostics and outside changes do not filter. In those lists, `j` and `k` move
too. The picker `Ctrl` keys use `Ctrl` on macOS too.

### Command palette

The command palette runs any command by name. It holds every item of the application menu,
so it is the menu in the terminal editor. It holds the same commands as the [leader menu](#leader-menu). Each row
shows its keys in the keymap you use: in the Neovim keymap, its own keys first, then the
leader keys. Type to filter.

These keys open it:

| Keys | Opens the palette |
| --- | --- |
| `Ctrl ⇧ P` | In both keymaps, from a file, the tree or no file. `⇧ ⌘ P` on macOS. Again closes it |
| `SPC p` | In the Neovim keymap |
| `:Commands` | In the Neovim keymap |

It has these commands:

| Command | Does |
| --- | --- |
| `Find file…` | Find a file |
| `Open project…`, `Open folder…` | Open a project from the list, or from the system's folder dialog |
| `Toggle file tree` | Show or hide the file tree |
| `Find in project…` | Search the project |
| `Find in file…`, `Replace in file…` | Open the [find bar](#find-and-replace), with its replace field for Replace |
| `Buffers…` | Pick an open buffer |
| `Save`, `Save all` | Write the buffer, every modified buffer |
| `Close tab` | Close the tab |
| `Quit` | Quit. A modified buffer stops it |
| `Diagnostics…` | List the diagnostics of this file |
| `Open settings…` | Open Settings |
| `App Setup…` | Open App Setup, from the config in use |
| `Show every key…` | Show every key, with search |
| `Git: next change`, `Git: previous change` | Next, previous git change against `HEAD` |
| `Git: status in the tree` | Show and focus the file tree, with its git status letters |
| `Git: toggle line blame` | Show who last changed each line and when, after its text. The cursor's line also shows the commit's summary |
| `Go to definition`, `Find references…`, `Hover`, `Rename symbol…`, `Code actions…` | Ask the language server |
| `Agent: list outside changes…` | List the changes made on disk from outside the editor |
| `Agent: clear change marks` | Clear the outside-change marks |
| `Agent: show the next proposal` | Show the next agent proposal |
| `Agent: clear the note` | Clear the agent's note |
| `Next tab`, `Previous tab` | Go to the next, the previous tab |
| `Close other tabs` | Close every tab but this one |
| `Close tabs to the left`, `Close tabs to the right` | Close the tabs before, after this one |
| `Close clean tabs`, `Close all tabs` | Close the tabs with no unsaved changes, every tab |
| `Pane: split right`, `Pane: split down` | Move this tab to a new pane on the right, below |
| `Pane: focus next`, `Pane: focus previous` | Give the keys to the next, the previous pane |
| `Pane: close` | Close the pane. Its tabs join the pane next to it |
| `Undo`, `Redo` | Undo or redo one change |
| `Cut`, `Copy`, `Paste`, `Select all` | The clipboard, on the selection or the Visual selection |
| `Toggle soft wrap` | Turn soft wrap on or off |
| `Keymap: Neovim`, `Keymap: classic` | Switch the keymap |
| `Theme: dark`, `Theme: light`, `Theme: follow the OS` | Switch the theme |

### Key list

`F1` shows every key of the keymap you use, in a picker. In the Neovim keymap, `SPC ?` and
`:Keys` show it too, and `:h dd` opens it with `dd` in the search line. In the file tree, `?`
shows it. The list comes from this file, so it always shows the keys that this file
describes.

Type words to search. A row stays when it holds every word, in its keys, in what it does or
in its section name. Case counts only when you type an uppercase letter, as with `smartcase`.
`Enter`, `Esc` or `F1` closes the list.

### Settings

`:Settings` or `SPC ,` in the Neovim keymap, and `Ctrl ,` in the classic keymap, open
Settings over the whole window. A change applies at once and goes to `config.toml` at once.
Settings takes every key while it shows. `:`, the leader and the window shortcuts do nothing
there.

| Keys | Does |
| --- | --- |
| `j` `↓`, `k` `↑` | Go to the next, previous option |
| `h` `←`, `l` `→` | Change the option back, forward |
| `Enter` | Change the option forward |
| `Space` | Change the option forward. Classic keymap |
| `Tab` `J`, `⇧ Tab` `K` | Go to the next, previous section |
| `1` to `5` | Go to a section: Appearance, Font, Editor, Formatting, Keys & mouse |
| `Esc`, `Ctrl W` (`⌘ W` on macOS) | Close Settings |
| `Ctrl Q` | Quit. Classic keymap |

### Setup

The first launch runs setup in five steps. To change those picks later, run `:Setup`, press
`SPC u S`, pick `App Setup…` in the command palette or the
app menu. Each step then starts from the config in use.

| Keys | Does |
| --- | --- |
| `j` `↓` `l` `→` | Go to the next choice |
| `k` `↑` `h` `←` | Go to the previous choice |
| `Enter` `Space` | Pick the choice |
| `Tab` | Go to the next step |
| `⇧ Tab` `Esc` | Go to the previous step. On the first step of setup run again, `Esc` leaves it with nothing changed |
| Chars, `Backspace` | Type, delete the name of the terminal command, on the name field |

The `skip`, `back` and `next →` buttons on the bottom line do the same with the mouse. On the
last step, click `Open NsVim`, or go to the last choice and press `Enter`. The editor then
writes `config.toml` and opens. When setup runs again, the last button is `save →`, and
`cancel` goes back to the editor with nothing changed.

### Agent proposals

An agent can propose a change over the control socket. The editor then shows the change as a
diff in place of the text. These keys work while the diff shows.

| Keys | Does |
| --- | --- |
| `y` `Enter` | Accept the change, as one undo step |
| `n` `Esc` | Reject the change |
| `]` `Tab`, `[` | Go to the next, previous changed block. `]c` and `[c` work too |
| `j` `↓`, `k` `↑` | Scroll one line |
| `Ctrl d` `PageDown`, `Ctrl u` `PageUp` | Scroll half a screen |
| `g`, `G` | Go to the top, the bottom |

Other keys, `:` included, show the hint on the status line again. `SPC a p` shows the next
proposal. When the agent leaves a note on lines, `Esc` outside Insert mode clears it, and so
does `SPC a n`.

### Completion list

The language server's completion list opens when you type the second char of a word. It also
opens after `.` and `::`. More chars filter it.

| Keys | Does |
| --- | --- |
| `↓` `Ctrl n`, `↑` `Ctrl p` | Go to the next, previous item |
| `⇧ Tab` | Go to the previous item |
| `Enter` `Tab` | Insert the item |
| `Ctrl y` | Insert the item. Neovim keymap |
| `Ctrl e` | Close the list. Neovim keymap |
| `Esc` | Close the list. In the Neovim keymap, also go back to Normal mode |

Other keys close the list or type as usual.

### Hover

`K` and `SPC l h` show the language server's information about the symbol under the cursor.
Any key closes it. `Esc` does nothing more. Other keys then do their usual work too.

### Context menu

When `mouse.right_click` is on, a right-click in the text opens a menu. It has cut, copy,
paste and select all. It also has go to definition, find references, rename symbol, code
actions and find in project. macOS and Windows show the system's menu. On Linux, these keys work in the menu:

| Keys | Does |
| --- | --- |
| `j` `↓` `Tab`, `k` `↑` | Go to the next, previous item |
| `Enter` `Space` | Run the item |
| Any other key | Close the menu |

### No file open

With no file open, the Neovim keymap takes the leader, `:`, `Ctrl ⇧ P` and `Ctrl w h`. The classic keymap
takes `Ctrl P`, `Ctrl ⇧ P`, `Ctrl ⇧ F`, `Ctrl B`, `Ctrl ⇧ E`, `Ctrl ,` and `Ctrl Q`, which quits.

## Mouse

Turn the mouse on or off in setup, in Settings or with `:set mouse`. When it is on:

- A click places the cursor. A drag selects, in Visual mode in the Neovim keymap. A drag
  past the top or bottom edge scrolls.
- `⇧`-click selects from the cursor to the click.
- A double-click selects a word. A triple-click, or a click on a line number, selects a line.
  A drag after a triple-click selects whole lines.
- The wheel scrolls. With `⇧`, it scrolls sideways when soft wrap is off. `mouse.scroll`
  controls the wheel on its own, so the wheel can work while the mouse is off.
- A right-click opens the context menu, when `mouse.right_click` is on.
- A click on a tab goes to it. A click on its `×`, or a middle-click on the tab, closes it.
  A tab with unsaved changes stays open.
- A right-click on a tab opens its menu. See [Tab menu](#tab-menu).
- Drag a tab along the line to move it. A line shows where it lands.
- A click on a row of the file tree opens the file or folder. The wheel over the tree scrolls
  it, and the keyboard row stays where it was. Drag the tree's edge to resize it.
- A click on a row of a picker or the leader menu runs it.
- In Settings, a click picks a section, changes an option or writes `config.toml` with the
  `:w` button on the status line.
- In the classic keymap, `Alt`-click adds a cursor.

`mouse.click`, `mouse.drag` and `mouse.double_click` in `config.toml` turn each of these off
on its own.

### Tab menu

A right-click on a tab opens its menu. The names and the order come from Zed. It does not
need `mouse.right_click`.

| Item | Does |
| --- | --- |
| `Close` | Close this tab |
| `Close others` | Close every tab but this one |
| `Close left`, `Close right` | Close the tabs before, after this one |
| `Close clean` | Close every tab with no unsaved changes |
| `Close all` | Close every tab |
| `Split right`, `Split down` | Move this tab to a new pane on the right, below |
| `Copy path`, `Copy relative path` | Copy the file's path, or its path in the project |
| `Reveal in file tree` | Show the file in the file tree |

A tab with unsaved changes stays open, and the status line names it. `Close clean` leaves
those tabs alone and says nothing. The tab that showed
still shows when it stays open. The palette and the `SPC t` keys do the same from the tab
that shows.

## Open a project

You can start NsVim first and pick the project after. `SPC o` (`Ctrl O` in the classic
keymap, `⌘ O` on macOS, `:Projects`, or `Open project…` in the palette) lists:

- `Open folder…`: the system's folder dialog.
- The recent projects, newest first. A project is recent once NsVim has closed in it.

Type to filter, `Enter` opens. The project opens as `nvim <folder>` does: in the window
already running for it, or in a new one. A window with no file open closes and gives its
place to the new one. A window with files open stays.

In a window, the application menu has the same: File › Open Project…, Open Folder… and Open
Recent, with the recent projects as they were when the window opened.

## Application menu

A window has an application menu. On macOS it is the menu bar. On Windows and Linux NsVim draws
its own title bar, and the system then shows no menu bar. There the menu button at the left end
of the title bar opens the same menus, one submenu each. The terminal editor has no menu. The
[command palette](#command-palette) holds every item of the menu, so use it there.

On macOS, Close Window hides NsVim as `⌘ H` does, and does not quit it: your files, tabs
and undo stay as they were. Click the Dock icon or pick NsVim with `⌘ Tab` to show the
window again. Its shortcut is `⌘ W` with the Neovim keymap. With the classic keymap `⌘ W`
closes the tab, so Close Window is `⇧ ⌘ W`, as in VS Code. While Settings shows, `⌘ W`
closes Settings. To quit, use `⌘ Q`, `:qa` or the close button. `⌘ Q` works in every keymap
and on every screen. Over unsaved edits it asks first, as the close button does: press it
again to quit and keep the edits for recovery.

- **NsVim**: Check for Updates…, Settings…, App Setup…, Quit. On macOS also Hide NsVim (`⌘ H`),
  Hide Others and Show All.
- **File**: New File, Open Project…, Open Folder…, Open Recent, Find File…, Save, Save All, Close Tab.
  On macOS also Close Window.
- **Edit**: Undo, Redo, Cut, Copy, Paste, Select All, Find…, Replace…, Find in Project….
- **View**: Command Palette…, Toggle File Tree, Toggle Soft Wrap, Next Tab, Previous Tab, Split Right,
  Split Down, Focus Next Pane, Focus Previous Pane, Close Pane.
- **Go**: Go to Definition, Find References…, Rename Symbol…, Code Actions….
- **Help**: Keyboard Shortcuts….

The items show no keys: the keymap's own keys stay as they are.
