# Configuration

nsvim reads `~/.config/nsvim/config.toml` at startup. On the first start, nsvim copies an older
`~/.config/editor` folder there once. The first launch writes it, at the
end of setup. `:Setup` runs setup again, from this file. Settings (`:Settings`) edits it over the whole window. A change applies at once and
goes to the file at once. A missing key takes its default, so a file can hold only what you change.

## Every option

```toml
[theme]
mode = "dark"            # light, dark or system
dark = "dark"            # the theme for dark mode, and for system mode on a dark OS
light = "light"          # the theme for light mode, and for system mode on a light OS

[font]
family = "JetBrains Mono"  # any installed monospace font
size = 13.0              # points, 9 to 24
line_height = 1.7        # a multiple of the size, 1.0 to 2.4
letter_spacing = 0.0     # points, -1.0 to 2.0
weight = 400             # 400, 500 or 700
ligatures = true         # join -> != => into single glyphs

[ui]
font = "editor"          # the tree, tabs and status line: "editor" or "system"
size = 12.5              # points
tree_width = 260.0       # points

[window]
opacity = 100            # 70 to 100. Below 100 lets a compositor's blur show. Next launch

[editor]
numbers = "hybrid"       # hybrid, relative, absolute or off
cursor = "block"         # block, bar or underline. Insert mode always draws a bar
cursorline = true
ruler = 100              # a guide at this column. 0 for none
whitespace = false       # leading spaces as dots
wrap = false             # soft-wrap long lines
scrolloff = 8            # lines kept around the cursor

[indent]
width = 4
spaces = true            # Tab inserts spaces

[format]
trim_whitespace = true   # on save
final_newline = true     # on save
line_endings = "lf"      # lf or crlf, for new files

[keys]
map = "neovim"           # neovim or classic
leader = "space"         # space or backslash. Neovim keymap only
import_nvim = true       # read the leader from ~/.config/nvim at setup
multi_cursor = true      # classic keymap: Alt-click adds a cursor
auto_close = true        # typing ( also types )

[mouse]
enabled = true
scroll = true
click = true
drag = true
double_click = true
right_click = false      # right-click opens a context menu

[cli]                    # what setup's terminal step did
editor_env = true        # $EDITOR and $VISUAL name the program by its full path
git = true               # git's editor, by the full path too
default_app = false      # Linux: the default app for the file types in editor.desktop
vim_aliases = true       # nvim, vim and vi open nsvim (shell aliases), \nvim runs Neovim
```

## Themes

| Dark | Light |
| --- | --- |
| `dark` (nsvim dark) | `light` (nsvim light) |
| `catppuccin` (Catppuccin Mocha) | `catppuccin-latte` |
| `tokyo-night` | `tokyo-night-day` |
| `gruvbox` | `gruvbox-light` |
| `kanagawa` | `rose-pine-dawn` |
| `rose-pine` | `one-light` |
| `one-dark` | |

On [Omarchy](https://omarchy.org), `theme.mode = "system"` follows the desktop theme. nsvim
reads `colors.toml` from `~/.local/state/omarchy/current/theme` (or
`~/.config/omarchy/current/theme` on older releases), and falls back to the theme's
`alacritty.toml`. A theme with a `light.mode` file, or `mode = "light"`, gets the light variant.
When you switch the desktop theme, nsvim changes with it. An older config with
`theme.mode = "omarchy"` still loads and works as `system`.

## .editorconfig

nsvim reads every `.editorconfig` from a file's folder up to the first one with `root = true`.
Nearer files win, and they win over config.toml.

| Key | Sets |
| --- | --- |
| `indent_style` | `space` or `tab` |
| `indent_size`, `tab_width` | Columns per indent |
| `end_of_line` | `lf` or `crlf` on save |
| `trim_trailing_whitespace` | Trim on save |
| `insert_final_newline` | Add a final newline on save |

## Options for this run

`:set` changes an option until you quit: `:set wrap`, `:set nonumber`, `:set tabstop=2`,
`:set keymap=classic`, `:set theme=light`, `:set theme=gruvbox`. `theme` takes a mode or a theme id
from the list above. A change in Settings goes to `config.toml`.
