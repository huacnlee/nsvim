# Install NSVim

Releases are at <https://github.com/huacnlee/nsvim/releases>. The installers download a release
and verify it with the release's `SHA256SUMS` before they install it. They run as the current
user and do not need administrator access.

Release builds are for:

- macOS 14 or later, Apple silicon and Intel.
- Linux x86_64 and ARM64, with GTK 4.14 or later.
- Windows x86_64.

## macOS and Linux

```sh
curl -fsSL https://huacnlee.github.io/nsvim/install.sh | sh
```

The same POSIX shell installer finds macOS or Linux and the CPU. Give options after `sh -s --`:

```sh
curl -fsSL https://huacnlee.github.io/nsvim/install.sh | sh -s -- --version 0.4.0 --dir ~/opt
```

| Option | Variable | Default |
| --- | --- | --- |
| `--version VERSION` | `NSVIM_VERSION` | the latest release |
| `--dir DIRECTORY` | `NSVIM_INSTALL_DIR` | `~/Applications` on macOS, `~/.local` on Linux |

To upgrade, run the installer again. Close NSVim first.

### What the installer puts on macOS

- `~/Applications/nsvim.app`: the app. Open it from Finder or Launchpad. The program is
  `~/Applications/nsvim.app/Contents/MacOS/nsvim`.

The release app has an ad hoc signature. It does not have a Developer ID signature, and Apple did
not notarize it. `curl` does not mark the download as quarantined, so macOS opens the app. The
installer does not change Gatekeeper.

### What the installer puts on Linux

`--dir` is a prefix. With the default prefix `~/.local`:

- `~/.local/share/nsvim/`: the program `nsvim` and its interface in `nsh/`.
- `~/.local/share/applications/nsvim.desktop`: the entry for the app launcher.
- `~/.local/share/icons/hicolor/scalable/apps/nsvim.svg`: the icon.

With another prefix, put its `share` folder on `XDG_DATA_DIRS`, so that launchers find the
desktop entry. Install GTK 4.14 or later from your distribution. The installer does not change
your package manager or your shell configuration.

The program reads its interface from `nsh/bundle.nshc` next to it. Keep the folder whole.

### The first launch

The installer puts no command on `PATH`. Open NSVim the first time from the app launcher (Finder
on macOS), or run the program by the path that the installer prints:

```sh
~/.local/share/nsvim/nsvim .
```

The first launch runs a short setup. Its terminal step adds shell aliases, so `nvim`, `vim` and
`vi` open nsvim. `\nvim` still runs Neovim. The step also sets `$EDITOR` and git's editor. Each
of them names the program by its full path, so an upgrade in the same folder keeps them working.
Open a new terminal after setup, then:

```sh
nvim .                  # this folder as the project
nvim src/main.rs:42     # a file, at line 42
```

Scripts and AI agents do not see shell aliases. They run the program by its full path.
`nvim --help` prints it.

## Windows

```powershell
irm https://huacnlee.github.io/nsvim/install.ps1 | iex
```

To give options, download the script and run it:

```powershell
Invoke-WebRequest https://huacnlee.github.io/nsvim/install.ps1 -OutFile "$env:TEMP\nsvim-install.ps1"
& "$env:TEMP\nsvim-install.ps1" -Version 0.4.0 -InstallDir "$env:LOCALAPPDATA\Programs\nsvim"
```

`NSVIM_VERSION` and `NSVIM_INSTALL_DIR` set the same defaults. The default folder is
`%LOCALAPPDATA%\Programs\nsvim`. The installer replaces the folder as one unit and adds a Start
menu shortcut. It does not change `PATH`. The program is `nsvim.exe` in that folder.

To upgrade, close NSVim and run the installer again. If the PowerShell policy stops scripts,
download the Windows ZIP from the release page and extract it.

## mise

mise can install the release archive, but it puts the program in a folder with the version in
its name. The aliases name that folder, so they break after `mise upgrade`. Use the installers.

## Build from source

You need Rust, the [NSH CLI](https://nsh.dev/docs/installation) 0.7, and on Linux the GTK 4
development packages (`libgtk-4-dev` on Debian and Ubuntu).

```sh
git clone https://github.com/huacnlee/nsvim && cd nsvim
cargo build --release --locked
nsh build --release -o target/release/nsh
target/release/nsvim .
```

`cargo build` makes the program `target/release/nsvim`. `nsh build` compiles the interface
(`App.tsx`) to `nsh/bundle.nshc` next to it. `cargo install` alone does not make a program that
runs, because it installs only the executable.

On macOS, the program reads the interface from `Resources/nsh` two folders up, as in an app
bundle. Write it with `nsh build --release -o target/Resources/nsh`.

## Release archives

Each release has these archives and one `SHA256SUMS` file:

| Archive | Contents |
| --- | --- |
| `nsvim-VERSION-x86_64-unknown-linux-gnu.tar.gz` | `nsvim`, `nsh/`, `packaging/nsvim.desktop`, `packaging/nsvim.svg`, `LICENSE`, `README.md` |
| `nsvim-VERSION-aarch64-unknown-linux-gnu.tar.gz` | the same, for ARM64 |
| `nsvim-VERSION-aarch64-apple-darwin.tar.gz` | `nsvim.app` (the interface is in `Contents/Resources/nsh/`), `LICENSE`, `README.md` |
| `nsvim-VERSION-x86_64-apple-darwin.tar.gz` | the same, for Intel |
| `nsvim-VERSION-x86_64-pc-windows-msvc.zip` | `nsvim.exe`, `nsh\`, `LICENSE`, `README.md` |

To install by hand, download the archive for your system, check its SHA-256 against
`SHA256SUMS`, and extract it. Keep `nsh/` next to the program.

## A link from an earlier setup

An earlier setup made the command `nsvim` as a link in `~/.local/bin`. The installer does not
touch it. To remove it, run `rm ~/.local/bin/nsvim`.

## Uninstall

Close NSVim, then remove what the installer added:

- macOS: `~/Applications/nsvim.app`.
- Linux: under the prefix, `share/nsvim/`, `share/applications/nsvim.desktop` and
  `share/icons/hicolor/scalable/apps/nsvim.svg`.
- Windows: the install folder and the NSVim Start menu shortcut.

Your settings, session, undo history and recovery files stay. On Linux, they are in
`~/.config/nsvim` and `~/.local/state/nsvim`. The first-run setup can add lines to your shell startup file
(after `# Added by nsvim's setup`) and set git's editor. Remove them yourself if you no longer
want them.
