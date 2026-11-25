//! First launch: pick keys, mouse, look and terminal integration, then open the editor.
//! The choices land in config.toml. Terminal integration runs only what the user leaves on,
//! and shows each command before it runs.

use std::collections::HashMap;
use std::path::PathBuf;

use super::fx::Tx;
use editor::editor::{KeyCode, KeyInput};

use super::{App, Screen};
use crate::config::Config;
use crate::theme::Omarchy;

pub const STEPS: &[&str] = &["keys", "mouse", "look", "terminal", "ready"];

/// The desktop entry and icon the default-app choice installs on Linux.
const DESKTOP: &str = include_str!("../../../assets/editor.desktop");
const ICON: &str = include_str!("../../../assets/editor.svg");
pub const FONTS: &[&str] = &["JetBrains Mono", "IBM Plex Mono", "Fira Code"];
pub const SIZES: &[f32] = &[12.0, 13.0, 14.0, 16.0];
pub const LOOKS: &[&str] = &["light", "dark", "system", "omarchy"];

pub struct Setup {
    pub step: usize,
    pub cfg: Config,
    /// Show macOS keys (⌘) rather than Linux and Windows ones (Ctrl).
    pub mac: bool,
    /// The focused choice on this step.
    pub focus: usize,
    pub shadow: bool,
    /// Whether `~/.local/bin` is on `$PATH`. When it is not, the terminal step adds it.
    pub bin_on_path: bool,
    /// This binary, with links resolved. The command link is not made when it is the binary.
    pub exe: Option<PathBuf>,
    /// `command -v` answers by name, so each name is looked up once.
    pub on_path: HashMap<String, Option<String>>,
    pub omarchy_name: Option<String>,
}

/// One choice on a step, for keys and the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Keymap(bool),
    Platform(bool),
    Toggle(&'static str),
    Mouse(bool),
    Look(usize),
    Font(usize),
    Size(usize),
    Name,
    Suggest(usize),
    Shadow,
    Change(usize),
    Open,
}

impl Setup {
    pub fn new(omarchy: Option<&Omarchy>) -> Setup {
        Setup {
            step: 1,
            cfg: Config::default(),
            mac: cfg!(target_os = "macos"),
            focus: 0,
            shadow: false,
            bin_on_path: bin_on_path(),
            exe: std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()),
            on_path: HashMap::new(),
            omarchy_name: omarchy.map(|o| o.name.clone()),
        }
    }

    pub fn vim(&self) -> bool {
        self.cfg.keys.map != "classic"
    }

    pub fn mod_key(&self) -> &'static str {
        if self.mac { "⌘" } else { "Ctrl" }
    }

    /// The choices on the current step, in focus order.
    pub fn choices(&self) -> Vec<Choice> {
        let mut c = Vec::new();
        match self.step {
            1 => {
                c.extend([Choice::Keymap(true), Choice::Keymap(false), Choice::Platform(true), Choice::Platform(false)]);
                if self.vim() {
                    c.extend([Choice::Toggle("leader"), Choice::Toggle("import")]);
                } else {
                    c.extend([Choice::Toggle("multi"), Choice::Toggle("autoclose")]);
                }
            }
            2 => {
                c.extend([Choice::Mouse(true), Choice::Mouse(false)]);
                if self.cfg.mouse.enabled {
                    c.extend([Choice::Toggle("click"), Choice::Toggle("drag"), Choice::Toggle("double"), Choice::Toggle("right")]);
                }
            }
            3 => {
                c.extend((0..LOOKS.len()).map(Choice::Look));
                c.extend((0..FONTS.len()).map(Choice::Font));
                c.extend((0..SIZES.len()).map(Choice::Size));
            }
            4 => {
                if self.cfg.cli.path {
                    c.push(Choice::Name);
                    if self.taken().is_some() {
                        c.extend((0..self.suggestions().len()).map(Choice::Suggest));
                        c.push(Choice::Shadow);
                    }
                }
                c.extend([Choice::Toggle("path"), Choice::Toggle("env"), Choice::Toggle("git")]);
                // The release ships no macOS app bundle, so only Linux can be the default app.
                if !self.mac {
                    c.push(Choice::Toggle("default"));
                }
            }
            _ => {
                c.extend((0..5).map(Choice::Change));
                c.push(Choice::Open);
            }
        }
        c
    }

    pub fn toggle_on(&self, key: &str) -> bool {
        let k = &self.cfg.keys;
        let m = &self.cfg.mouse;
        let cli = &self.cfg.cli;
        match key {
            "leader" => k.leader == "space",
            "import" => k.import_nvim,
            "multi" => k.multi_cursor,
            "autoclose" => k.auto_close,
            "click" => m.click,
            "drag" => m.drag,
            "double" => m.double_click,
            "right" => m.right_click,
            "path" => cli.path,
            "env" => cli.editor_env,
            "git" => cli.git,
            "default" => cli.default_app,
            _ => false,
        }
    }

    pub fn flip(&mut self, key: &str) {
        let on = !self.toggle_on(key);
        let k = &mut self.cfg.keys;
        let m = &mut self.cfg.mouse;
        let cli = &mut self.cfg.cli;
        match key {
            "leader" => k.leader = if on { "space".into() } else { "backslash".into() },
            "import" => k.import_nvim = on,
            "multi" => k.multi_cursor = on,
            "autoclose" => k.auto_close = on,
            "click" => m.click = on,
            "drag" => m.drag = on,
            "double" => m.double_click = on,
            "right" => m.right_click = on,
            "path" => cli.path = on,
            "env" => cli.editor_env = on,
            "git" => cli.git = on,
            "default" => cli.default_app = on,
            _ => {}
        }
    }

    pub fn name_valid(&self) -> bool {
        let n = &self.cfg.cli.name;
        !n.is_empty()
            && n.len() <= 32
            && n.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
    }

    /// Where the name already is on `$PATH`, if anywhere.
    pub fn taken(&self) -> Option<String> {
        if !self.name_valid() {
            return None;
        }
        self.on_path.get(&self.cfg.cli.name).cloned().flatten()
    }

    /// Looks the current name up on `$PATH`, once per name.
    pub fn check_name(&mut self) {
        let n = self.cfg.cli.name.clone();
        if self.name_valid() && !self.on_path.contains_key(&n) {
            self.on_path.insert(n.clone(), command_v(&n));
        }
        for s in ["edtr", "e", "edx", "ered", "medit"] {
            if !self.on_path.contains_key(s) {
                self.on_path.insert(s.to_string(), command_v(s));
            }
        }
    }

    /// Up to three free names.
    pub fn suggestions(&self) -> Vec<&'static str> {
        ["edtr", "e", "edx", "ered", "medit"]
            .into_iter()
            .filter(|n| *n != self.cfg.cli.name && self.on_path.get(*n).is_some_and(|p| p.is_none()))
            .take(3)
            .collect()
    }

    /// Whether the terminal command can be installed under its name.
    pub fn installable(&self) -> bool {
        self.name_valid() && (self.taken().is_none() || self.shadow)
    }

    /// The folder for the terminal command. `/usr/local/bin` needs root on a new Mac, so both
    /// systems use the user's own folder.
    pub fn bin_dir(&self) -> String {
        "~/.local/bin".into()
    }

    /// Whether this binary is the terminal command already: `~/.local/bin/<name>`, as the README
    /// installs it. A link there would replace the binary with a link to itself.
    pub fn in_bin(&self) -> bool {
        let (Some(exe), Some(home)) = (&self.exe, dirs::home_dir()) else { return false };
        *exe == home.join(".local/bin").join(&self.cfg.cli.name)
    }

    /// Whether the terminal step adds `~/.local/bin` to the front of `$PATH`: when it is not on
    /// `$PATH`, or when editor's command must come before a program with the same name.
    pub fn adds_path(&self) -> bool {
        !self.bin_on_path || (self.taken().is_some() && self.shadow)
    }

    pub fn rc_file(&self) -> String {
        let shell = std::env::var("SHELL").unwrap_or_default();
        if shell.ends_with("zsh") { "~/.zshrc".into() } else { "~/.bashrc".into() }
    }

    /// The commands the terminal step runs, as the user reads them.
    pub fn commands(&self) -> Vec<String> {
        let exe = std::env::current_exe().map(|p| crate::app::tilde(&p)).unwrap_or_else(|_| "editor".into());
        let name = &self.cfg.cli.name;
        let cli = &self.cfg.cli;
        let cmd_ref = if cli.path && self.installable() { name.clone() } else { exe.clone() };
        let mut out = Vec::new();
        if cli.path && self.installable() {
            if !self.in_bin() {
                out.push(format!("mkdir -p {0} && ln -sf {exe} {0}/{name}", self.bin_dir()));
            }
            if self.adds_path() {
                out.push(format!("echo 'export PATH=\"{}:$PATH\"' >> {}", self.bin_dir().replace('~', "$HOME"), self.rc_file()));
            }
        }
        if cli.editor_env {
            out.push(format!("echo 'export EDITOR=\"{cmd_ref} --wait\" VISUAL=\"{cmd_ref} --wait\"' >> {}", self.rc_file()));
        }
        if cli.git {
            out.push(format!("git config --global core.editor \"{cmd_ref} --wait\""));
        }
        if cli.default_app && !self.mac {
            out.push("# write ~/.local/share/applications/editor.desktop and its icon".into());
            out.push(format!("xdg-mime default editor.desktop {}", mime_types().join(" ")));
        }
        out
    }
}

/// Whether `~/.local/bin` is a folder on `$PATH`.
fn bin_on_path() -> bool {
    let (Some(path), Some(home)) = (std::env::var_os("PATH"), dirs::home_dir()) else { return false };
    let bin = home.join(".local/bin");
    std::env::split_paths(&path).any(|d| d == bin)
}

/// The file types the desktop entry opens: its `MimeType=` line.
fn mime_types() -> Vec<&'static str> {
    DESKTOP.lines().find_map(|l| l.strip_prefix("MimeType=")).map(|m| m.split(';').filter(|t| !t.is_empty()).collect()).unwrap_or_default()
}

/// `command -v name`: the first executable `name` on `$PATH`.
fn command_v(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let found = std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file())?;
    // This editor, installed under that name (`cargo install`, an earlier setup): not taken.
    let me = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok());
    if me.is_some() && found.canonicalize().ok() == me {
        return None;
    }
    let shown = crate::app::tilde(&found);
    Some(match std::fs::read_link(&found) {
        Ok(target) => format!("{shown}, a link to {}", crate::app::tilde(&target)),
        Err(_) => shown,
    })
}

impl App {
    pub fn setup_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let Screen::Setup(s) = &mut self.screen else { return };
        let choices = s.choices();
        let focus = s.focus.min(choices.len().saturating_sub(1));
        let on_name = choices.get(focus) == Some(&Choice::Name);
        match k.code {
            KeyCode::Char(c) if on_name && !k.ctrl && !k.cmd && c != ' ' => {
                s.cfg.cli.name.push(c);
                s.shadow = false;
                s.check_name();
            }
            KeyCode::Backspace if on_name => {
                s.cfg.cli.name.pop();
                s.shadow = false;
                s.check_name();
            }
            KeyCode::Tab if k.shift => self.setup_go(-1),
            KeyCode::Tab => self.setup_go(1),
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('l') | KeyCode::Right => {
                s.focus = (focus + 1).min(choices.len().saturating_sub(1))
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('h') | KeyCode::Left => s.focus = focus.saturating_sub(1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(c) = choices.get(focus).copied() {
                    self.setup_choose(c, tx);
                }
            }
            KeyCode::Esc => self.setup_go(-1),
            _ => {}
        }
    }

    /// Moves `by` steps; past the last step, the editor opens.
    pub fn setup_go(&mut self, by: isize) {
        let Screen::Setup(s) = &mut self.screen else { return };
        let next = (s.step as isize + by).clamp(1, STEPS.len() as isize) as usize;
        s.step = next;
        s.focus = 0;
        if next == 4 {
            s.check_name();
        }
    }

    pub fn setup_choose(&mut self, c: Choice, tx: &mut Tx) {
        let Screen::Setup(s) = &mut self.screen else { return };
        if let Some(i) = s.choices().iter().position(|x| *x == c) {
            s.focus = i;
        }
        match c {
            Choice::Keymap(vim) => s.cfg.keys.map = if vim { "neovim".into() } else { "classic".into() },
            Choice::Platform(mac) => s.mac = mac,
            Choice::Toggle(key) => s.flip(key),
            Choice::Mouse(on) => s.cfg.mouse.enabled = on,
            Choice::Look(i) => s.cfg.theme.mode = LOOKS[i].into(),
            Choice::Font(i) => s.cfg.font.family = FONTS[i].into(),
            Choice::Size(i) => s.cfg.font.size = SIZES[i],
            Choice::Name => {}
            Choice::Suggest(i) => {
                if let Some(n) = s.suggestions().get(i) {
                    s.cfg.cli.name = n.to_string();
                    s.shadow = false;
                }
            }
            Choice::Shadow => s.shadow = !s.shadow,
            Choice::Change(i) => {
                s.step = [1, 2, 3, 3, 4][i.min(4)];
                s.focus = 0;
            }
            Choice::Open => {
                self.finish_setup(tx);
                return;
            }
        }
        // The window follows the pick at once.
        if matches!(c, Choice::Look(_) | Choice::Font(_) | Choice::Size(_)) {
            self.cfg.theme = s.cfg.theme.clone();
            self.cfg.font = s.cfg.font.clone();
            self.font_family = crate::geometry::family(&self.cfg.font.family);
        }
    }

    /// Writes config.toml, runs the terminal steps the user left on, and opens the editor.
    pub fn finish_setup(&mut self, _tx: &mut Tx) {
        let Screen::Setup(s) = std::mem::replace(&mut self.screen, Screen::Editor) else { return };
        let cmds = s.commands();
        let integrate = Integration {
            name: s.cfg.cli.name.clone(),
            link: s.cfg.cli.path && s.installable() && !s.in_bin(),
            by_name: s.cfg.cli.path && s.installable(),
            add_path: s.cfg.cli.path && s.installable() && s.adds_path(),
            env: s.cfg.cli.editor_env,
            git: s.cfg.cli.git,
            default_app: s.cfg.cli.default_app && !s.mac,
            rc: s.rc_file(),
        };
        let mac = s.mac;
        self.cfg = s.cfg;
        let mut note = String::new();
        if self.cfg.keys.map == "neovim" && self.cfg.keys.import_nvim {
            match nvim_leader() {
                Some(' ') => self.cfg.keys.leader = "space".into(),
                Some('\\') => self.cfg.keys.leader = "backslash".into(),
                Some(c) => {
                    note = format!("Neovim's leader is '{c}'. editor takes Space or backslash, so it keeps {}. ", self.cfg.keys.leader)
                }
                None => {}
            }
        }
        self.config_changed();
        self.write_config();
        if !cmds.is_empty() {
            std::thread::spawn(move || integrate.run());
        }
        let m = if mac { "⌘" } else { "Ctrl" };
        let tip = if self.is_vim() {
            let l = if self.ed.vim.leader == ' ' { "SPC" } else { "\\" };
            format!("{l} for keys, {l} f to find a file, :Settings for settings")
        } else {
            format!("{m} P finds a file, {m} ⇧ P runs a command, {m} , opens settings")
        };
        self.ed.message(format!("{note}{tip}"));
    }
}

/// The leader key `~/.config/nvim` sets (`vim.g.mapleader` or `let mapleader`), if any. Reads
/// `init.lua`, `init.vim` and the Lua files under `lua/`, once, when setup finishes.
fn nvim_leader() -> Option<char> {
    let dir = dirs::config_dir()?.join("nvim");
    let mut files = vec![dir.join("init.lua"), dir.join("init.vim")];
    let mut stack = vec![dir.join("lua")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() && stack.len() < 64 {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "lua") && files.len() < 400 {
                files.push(p);
            }
        }
    }
    files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).find_map(|src| leader_in(&src))
}

/// The leader a Lua or Vimscript source sets.
fn leader_in(src: &str) -> Option<char> {
    for line in src.lines() {
        let l = line.trim();
        if l.starts_with("--") || l.starts_with('"') || !l.contains("mapleader") || l.contains("maplocalleader") {
            continue;
        }
        let (_, value) = l.split_once('=')?;
        let v = value.trim().trim_end_matches(';');
        let inner = v.trim_matches(|c| c == '"' || c == '\'');
        return match inner {
            "<Space>" | "\\<Space>" | " " => Some(' '),
            "\\" | "\\\\" => Some('\\'),
            s => s.chars().next(),
        };
    }
    None
}

/// The terminal step's changes, run off the UI thread after setup.
struct Integration {
    name: String,
    link: bool,
    /// `$EDITOR` and git name the command, not the binary's full path.
    by_name: bool,
    add_path: bool,
    env: bool,
    git: bool,
    default_app: bool,
    rc: String,
}

impl Integration {
    fn run(self) {
        let Ok(exe) = std::env::current_exe() else { return };
        let home = dirs::home_dir().unwrap_or_default();
        let expand = |p: &str| PathBuf::from(p.replacen('~', &home.to_string_lossy(), 1));
        let bin = home.join(".local/bin");
        let cmd_ref = if self.by_name { self.name.clone() } else { exe.display().to_string() };
        let dest = bin.join(&self.name);
        // Never remove the binary itself: the link would then point at itself.
        if self.link && dest.canonicalize().ok() != exe.canonicalize().ok() {
            let _ = std::fs::create_dir_all(&bin);
            let _ = std::fs::remove_file(&dest);
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(&exe, &dest);
        }
        let rc = expand(&self.rc);
        let append = |line: String| {
            let have = std::fs::read_to_string(&rc).unwrap_or_default();
            if !have.contains(&line) {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&rc) {
                    let _ = writeln!(f, "\n# Added by editor's setup\n{line}");
                }
            }
        };
        if self.add_path {
            append(format!("export PATH=\"{}:$PATH\"", bin.display()));
        }
        if self.env {
            append(format!("export EDITOR=\"{cmd_ref} --wait\" VISUAL=\"{cmd_ref} --wait\""));
        }
        if self.git {
            let _ = std::process::Command::new("git").args(["config", "--global", "core.editor", &format!("{cmd_ref} --wait")]).status();
        }
        if self.default_app {
            // A launcher may not have ~/.local/bin on its PATH, so the entry names the binary in full.
            let apps = home.join(".local/share/applications");
            let icons = home.join(".local/share/icons/hicolor/scalable/apps");
            let _ = std::fs::create_dir_all(&apps);
            let _ = std::fs::create_dir_all(&icons);
            let _ = std::fs::write(icons.join("editor.svg"), ICON);
            let _ = std::fs::write(apps.join("editor.desktop"), desktop_entry(&exe));
            let _ = std::process::Command::new("xdg-mime").arg("default").arg("editor.desktop").args(mime_types()).status();
        }
    }
}

/// The desktop entry with `Exec` set to this binary.
fn desktop_entry(exe: &std::path::Path) -> String {
    let exe = exe.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    DESKTOP.replace("Exec=editor %F", &format!("Exec=\"{exe}\" %F"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_entry_names_this_binary_and_every_file_type() {
        let entry = desktop_entry(std::path::Path::new("/home/dev/.local/bin/editor"));
        assert!(entry.contains("Exec=\"/home/dev/.local/bin/editor\" %F"), "{entry}");
        let types = mime_types();
        assert!(types.contains(&"text/plain") && types.contains(&"text/x-rust") && types.contains(&"text/markdown"), "{types:?}");
    }

    #[test]
    fn the_terminal_step_shows_what_it_runs() {
        let mut s = Setup::new(None);
        s.mac = false;
        s.bin_on_path = false;
        s.cfg.cli.default_app = true;
        let cmds = s.commands().join("\n");
        assert!(cmds.contains("mkdir -p ~/.local/bin && ln -sf"), "{cmds}");
        assert!(cmds.contains("export PATH=\"$HOME/.local/bin:$PATH\""), "a folder off $PATH is added: {cmds}");
        assert!(cmds.contains("xdg-mime default editor.desktop text/plain text/x-rust"), "{cmds}");
        s.bin_on_path = true;
        assert!(!s.commands().join("\n").contains("export PATH"), "{cmds}");
        s.mac = true;
        assert!(!s.commands().join("\n").contains("xdg-mime"), "macOS has no default-app step");
        s.step = 4;
        assert!(!s.choices().contains(&Choice::Toggle("default")), "and no default-app choice");

        // Installed as the README says: the binary is the command already, so no link replaces it.
        s.exe = dirs::home_dir().map(|h| h.join(".local/bin/editor"));
        let cmds = s.commands().join("\n");
        assert!(!cmds.contains("ln -sf"), "{cmds}");
        assert!(cmds.contains("export EDITOR=\"editor --wait\""), "{cmds}");
    }

    #[test]
    fn leaders_parse_from_lua_and_vimscript() {
        assert_eq!(leader_in("vim.g.mapleader = \" \"\nvim.g.maplocalleader = \"\\\\\""), Some(' '));
        assert_eq!(leader_in("let mapleader = \"\\<Space>\""), Some(' '));
        assert_eq!(leader_in("let mapleader = ','"), Some(','));
        assert_eq!(leader_in("vim.g.mapleader = '\\\\'"), Some('\\'));
        assert_eq!(leader_in("-- vim.g.mapleader = ' '"), None);
    }
}
