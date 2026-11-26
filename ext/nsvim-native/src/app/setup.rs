//! First launch: pick keys, mouse, look and terminal integration, then open the editor.
//! `:Setup` (and `Open setup wizard…` in the palette and the app menu) shows it again later, from
//! the current config, to change the picks. The choices land in config.toml. Terminal integration runs only what the user leaves on,
//! and shows each command before it runs.

use std::path::PathBuf;

use super::fx::Tx;
use editor::editor::{KeyCode, KeyInput};

use super::{App, Screen};
use crate::args::shell_word;
use crate::config::Config;
use crate::theme::Omarchy;

pub const STEPS: &[&str] = &["Keys", "Mouse", "Look", "Terminal", "Ready"];

/// The desktop entry and icon the default-app choice installs on Linux.
const DESKTOP: &str = include_str!("../../../../assets/editor.desktop");
#[cfg(not(target_arch = "wasm32"))]
const ICON: &str = include_str!("../../../../assets/editor.svg");
pub const FONTS: &[&str] = &["JetBrains Mono", "IBM Plex Mono", "Fira Code"];
pub const SIZES: &[f32] = &[12.0, 13.0, 14.0, 16.0];
/// The looks setup offers. System follows the Omarchy theme where there is one.
pub const LOOKS: &[&str] = &["system", "light", "dark"];

pub struct Setup {
    pub step: usize,
    pub cfg: Config,
    /// Show macOS keys (⌘) rather than Linux and Windows ones (Ctrl).
    pub mac: bool,
    /// The focused choice on this step.
    pub focus: usize,
    /// The terminal step shows. Windows has none: its shells do not read the lines it writes.
    pub terminal: bool,
    /// The default-app choice shows. Only Linux reads the desktop entry it writes.
    pub desktop_entry: bool,
    /// This is a development build: the NSH dev host, or a binary in a cargo `target` folder.
    /// The terminal step does not show, and nothing names this binary.
    pub dev_build: bool,
    /// This binary, with links resolved. The aliases, `$EDITOR` and git name it by its full path.
    pub exe: Option<PathBuf>,
    /// The leader key `~/.config/nvim` sets, as the probe worker found it.
    pub nvim_leader: Option<char>,
    pub omarchy_name: Option<String>,
    /// Setup runs again over the editor (`:Setup`): the config it started from. Cancel puts
    /// it back. `None` on first launch, when there is no editor to go back to.
    pub prior: Option<Box<Config>>,
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
    Change(usize),
    Open,
}

impl Setup {
    pub fn new(omarchy: Option<&Omarchy>) -> Setup {
        let mut cfg = Config::default();
        // On Omarchy, the look follows the desktop theme unless the user picks another.
        if omarchy.is_some() {
            cfg.theme.mode = "system".into();
        }
        Setup {
            step: 1,
            cfg,
            mac: cfg!(target_os = "macos"),
            focus: 0,
            terminal: !cfg!(target_os = "windows"),
            desktop_entry: cfg!(target_os = "linux"),
            // The probe worker also looks at the path of this binary (`App::setup_answer`).
            dev_build: dev_build(None),
            // The probe worker answers this from the disk (`App::setup_answer`).
            exe: None,
            nvim_leader: None,
            omarchy_name: omarchy.map(|o| o.name.clone()),
            prior: None,
        }
    }

    /// Setup again, from `cfg`: each step shows the current picks.
    pub fn again(cfg: &Config, omarchy: Option<&Omarchy>) -> Setup {
        Setup { cfg: cfg.clone(), prior: Some(Box::new(cfg.clone())), ..Setup::new(omarchy) }
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
            4 if self.terminal_step() => {
                // Setup puts no command on $PATH. The aliases open this binary by its full path.
                c.extend([Choice::Toggle("vim"), Choice::Toggle("env"), Choice::Toggle("git")]);
                // The default-app step writes a desktop entry, which only Linux reads.
                if self.desktop_entry {
                    c.push(Choice::Toggle("default"));
                }
            }
            _ => {
                // The rows of the summary: keys, mouse, theme, font and, with its step, terminal.
                c.extend((0..self.rows()).map(Choice::Change));
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
            "env" => cli.editor_env,
            "git" => cli.git,
            "default" => cli.default_app,
            "vim" => cli.vim_aliases,
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
            "env" => cli.editor_env = on,
            "git" => cli.git = on,
            "default" => cli.default_app = on,
            "vim" => cli.vim_aliases = on,
            _ => {}
        }
    }

    /// The terminal step shows. Windows and development builds have none.
    pub fn terminal_step(&self) -> bool {
        self.terminal && !self.dev_build
    }

    /// The rows of the summary on the last step.
    pub fn rows(&self) -> usize {
        if self.terminal_step() { 5 } else { 4 }
    }

    /// The steps that show, by name. Windows and development builds have no terminal step.
    pub fn steps(&self) -> Vec<&'static str> {
        STEPS.iter().copied().filter(|t| self.terminal_step() || *t != "Terminal").collect()
    }

    pub fn rc_file(&self) -> String {
        let shell = std::env::var("SHELL").unwrap_or_default();
        if shell.ends_with("zsh") { "~/.zshrc".into() } else { "~/.bashrc".into() }
    }

    /// This binary as the shell names it: its full path, quoted when it needs quotes.
    pub fn program(&self) -> String {
        let exe = self.exe.clone().or_else(|| std::env::current_exe().ok());
        exe.map(|p| shell_word(&p.display().to_string())).unwrap_or_else(|| "nsvim".into())
    }

    /// The commands the terminal step runs, as the user reads them.
    pub fn commands(&self) -> Vec<String> {
        let cli = &self.cfg.cli;
        let rc = self.rc_file();
        let program = self.program();
        let mut out = Vec::new();
        if !self.terminal_step() {
            return out;
        }
        if cli.vim_aliases {
            out.push(append_line(&vim_aliases(&program), &rc));
        }
        if cli.editor_env {
            out.push(append_line(&editor_env(&program), &rc));
        }
        if cli.git {
            out.push(format!("git config --global core.editor \"{}\"", in_double_quotes(&format!("{program} --wait"))));
        }
        if cli.default_app && self.desktop_entry {
            out.push("# write ~/.local/share/applications/nsvim.desktop and its icon".into());
            out.push(format!("xdg-mime default nsvim.desktop {}", mime_types().join(" ")));
        }
        out
    }
}

/// `text` escaped for a place inside double quotes in the shell.
fn in_double_quotes(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$").replace('`', "\\`")
}

/// The shell command that adds `line` to the startup file `rc`, as the user reads it.
fn append_line(line: &str, rc: &str) -> String {
    format!("echo {} >> {rc}", shell_word(line))
}

/// The startup-file line that sets `$EDITOR` and `$VISUAL` to `program`.
fn editor_env(program: &str) -> String {
    let value = in_double_quotes(&format!("{program} --wait"));
    format!("export EDITOR=\"{value}\" VISUAL=\"{value}\"")
}

/// The shell line that makes `nvim`, `vim` and `vi` open nsvim, the binary `program` (a shell
/// word). `\nvim` or `command nvim` still runs the real Neovim.
fn vim_aliases(program: &str) -> String {
    let p = in_double_quotes(program);
    format!("alias nvim=\"{p}\" vim=\"{p}\" vi=\"{p}\"")
}

/// The file types the desktop entry opens: its `MimeType=` line.
fn mime_types() -> Vec<&'static str> {
    DESKTOP.lines().find_map(|l| l.strip_prefix("MimeType=")).map(|m| m.split(';').filter(|t| !t.is_empty()).collect()).unwrap_or_default()
}

impl App {
    pub fn setup_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let Screen::Setup(s) = &mut self.screen else { return };
        let choices = s.choices();
        let focus = s.focus.min(choices.len().saturating_sub(1));
        match k.code {
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
            // On the first step of setup run again, Esc leaves it as it was.
            KeyCode::Esc if s.step == 1 && s.prior.is_some() => self.cancel_setup(),
            KeyCode::Esc => self.setup_go(-1),
            _ => {}
        }
    }

    /// Shows setup again over the editor, from the current config (`:Setup`).
    pub fn open_setup(&mut self) {
        if matches!(self.screen, Screen::Setup(_)) {
            return;
        }
        self.close_settings();
        self.overlay = super::Overlay::None;
        self.ed.mode = self.ed.rest_mode();
        self.screen = Screen::Setup(Box::new(Setup::again(&self.cfg, self.omarchy.as_ref())));
        if let Some(svc) = &self.svc {
            svc.probe_setup();
        }
    }

    /// Leaves setup run again with nothing changed: the look it showed goes back too.
    pub fn cancel_setup(&mut self) {
        let Screen::Setup(s) = &self.screen else { return };
        let Some(prior) = s.prior.clone() else { return };
        self.screen = Screen::Editor;
        self.cfg = *prior;
        self.config_changed();
        self.ed.message("setup closed, nothing changed");
    }

    /// Moves `by` steps; past the last step, the editor opens.
    pub fn setup_go(&mut self, by: isize) {
        let Screen::Setup(s) = &mut self.screen else { return };
        let mut next = (s.step as isize + by).clamp(1, STEPS.len() as isize) as usize;
        // Without the terminal step, a move goes over it.
        if !s.terminal_step() && STEPS[next - 1] == "Terminal" {
            next = if by < 0 { next - 1 } else { next + 1 };
        }
        s.step = next;
        s.focus = 0;
    }

    /// The probe worker answered: keep what it found for the setup screen.
    pub fn setup_answer(&mut self, answer: crate::services::probe::Answer) {
        let Screen::Setup(s) = &mut self.screen else { return };
        let crate::services::probe::Answer::Facts { exe, dev_build, nvim_leader } = answer;
        s.exe = exe;
        // A development build must not put its own path in the user's shell, git or desktop.
        if dev_build {
            s.dev_build = true;
            s.cfg.cli.vim_aliases = false;
            s.cfg.cli.editor_env = false;
            s.cfg.cli.git = false;
            s.cfg.cli.default_app = false;
            if s.step == 4 {
                s.step = 5;
                s.focus = 0;
            }
        }
        s.nvim_leader = nvim_leader;
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
        // The web has no terminal to set up.
        #[cfg(not(target_arch = "wasm32"))]
        let cmds = s.commands();
        #[cfg(not(target_arch = "wasm32"))]
        let integrate = Integration {
            program: s.program(),
            env: s.cfg.cli.editor_env,
            vim: s.cfg.cli.vim_aliases,
            git: s.cfg.cli.git,
            default_app: s.cfg.cli.default_app && s.desktop_entry,
            rc: s.rc_file(),
        };
        let mac = s.mac;
        let nvim = s.nvim_leader;
        let again = s.prior.is_some();
        self.cfg = s.cfg;
        let mut note = String::new();
        if self.cfg.keys.map == "neovim" && self.cfg.keys.import_nvim {
            match nvim {
                Some(' ') => self.cfg.keys.leader = "space".into(),
                Some('\\') => self.cfg.keys.leader = "backslash".into(),
                Some(c) => {
                    note = format!("Neovim's leader is '{c}'. nsvim takes Space or backslash, so it keeps {}. ", self.cfg.keys.leader)
                }
                None => {}
            }
        }
        self.config_changed();
        self.write_config();
        #[cfg(not(target_arch = "wasm32"))]
        if !cmds.is_empty() {
            std::thread::spawn(move || integrate.run());
        }
        if again {
            self.ed.message(format!("{note}setup saved to config.toml"));
            return;
        }
        let m = if mac { "⌘" } else { "Ctrl" };
        let tip = if self.is_vim() {
            let l = if self.ed.vim.leader == ' ' { "SPC" } else { "\\" };
            format!("{l} for keys, {l} f to find a file, :Settings for settings, :Setup to run this again")
        } else {
            format!("{m} P finds a file, {m} ⇧ P runs a command, {m} , opens settings")
        };
        self.ed.message(format!("{note}{tip}"));
    }
}

/// Set by `treat_dev_build_as_release`.
static AS_RELEASE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Makes setup treat this development build as a release build. The integration tests run
/// from `target/debug`, and their setup must write to their own home as a release build does.
pub fn treat_dev_build_as_release() {
    AS_RELEASE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// This is a development build: the NSH dev host (`nsh run` sets `NSH_DEV_APP_DIR`), or the
/// binary `exe` is in a cargo `target` folder (`dev_path`).
pub fn dev_build(exe: Option<&std::path::Path>) -> bool {
    !AS_RELEASE.load(std::sync::atomic::Ordering::Relaxed) && (std::env::var_os("NSH_DEV_APP_DIR").is_some() || exe.is_some_and(dev_path))
}

/// `exe` is a development build: it is in a cargo `target` folder, as `target/debug/nsvim`,
/// `target/<triple>/release/nsvim`, the NSH dev host in `.nsh/rust/target/debug`, or a shared
/// target folder such as `~/.cargo/targets/<name>/debug`.
pub fn dev_path(exe: &std::path::Path) -> bool {
    let parts: Vec<_> = exe.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let profile = |i: usize| parts.get(i).is_some_and(|p| ["debug", "release", "quick"].contains(&p.as_str()));
    parts.iter().enumerate().any(|(i, p)| (p == "target" || p == "targets") && (i + 1..=i + 3).any(profile))
}

/// The leader a Lua or Vimscript source sets.
pub fn leader_in(src: &str) -> Option<char> {
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

#[cfg(not(target_arch = "wasm32"))]
/// The terminal step's changes, run off the UI thread after setup. Nothing goes on `$PATH`:
/// the aliases, `$EDITOR` and git name the binary by its full path.
struct Integration {
    /// This binary as a shell word (`Setup::program`).
    program: String,
    env: bool,
    /// `nvim`, `vim` and `vi` aliases in the shell's startup file.
    vim: bool,
    git: bool,
    default_app: bool,
    rc: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl Integration {
    fn run(self) {
        let Some(exe) = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()) else { return };
        // The last guard: setup may finish before the probe worker answers.
        if dev_build(Some(&exe)) {
            return;
        }
        let home = dirs::home_dir().unwrap_or_default();
        let rc = PathBuf::from(self.rc.replacen('~', &home.to_string_lossy(), 1));
        let append = |line: String| {
            let have = std::fs::read_to_string(&rc).unwrap_or_default();
            if !have.contains(&line) {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&rc) {
                    let _ = writeln!(f, "\n# Added by nsvim's setup\n{line}");
                }
            }
        };
        if self.vim {
            append(vim_aliases(&self.program));
        }
        if self.env {
            append(editor_env(&self.program));
        }
        if self.git {
            let editor = format!("{} --wait", self.program);
            let _ = std::process::Command::new("git").args(["config", "--global", "core.editor", &editor]).status();
        }
        if self.default_app {
            // A launcher may not have ~/.local/bin on its PATH, so the entry names the binary in full.
            let apps = home.join(".local/share/applications");
            let icons = home.join(".local/share/icons/hicolor/scalable/apps");
            let _ = std::fs::create_dir_all(&apps);
            let _ = std::fs::create_dir_all(&icons);
            let _ = std::fs::write(icons.join("nsvim.svg"), ICON);
            let _ = std::fs::write(apps.join("nsvim.desktop"), desktop_entry(&exe));
            let _ = std::process::Command::new("xdg-mime").arg("default").arg("nsvim.desktop").args(mime_types()).status();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// The desktop entry with `Exec` set to this binary.
fn desktop_entry(exe: &std::path::Path) -> String {
    let exe = exe.display().to_string().replace('\\', "\\\\").replace('"', "\\\"");
    DESKTOP
        .replace("Exec=editor %F", &format!("Exec=\"{exe}\" %F"))
        .replace("Name=editor", "Name=nsvim")
        .replace("Icon=editor", "Icon=nsvim")
        .replace("StartupWMClass=editor", "StartupWMClass=dev.nsvim.app")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terminal_step_aliases_nvim_vim_and_vi_to_the_full_path() {
        let mut s = Setup::new(None);
        s.step = 4;
        s.terminal = true;
        s.dev_build = false;
        s.exe = Some(PathBuf::from("/home/dev/.local/share/nsvim/nsvim"));
        let line = "alias nvim=\"/home/dev/.local/share/nsvim/nsvim\" vim=\"/home/dev/.local/share/nsvim/nsvim\" vi=\"/home/dev/.local/share/nsvim/nsvim\"";
        let alias = |s: &Setup| s.commands().iter().any(|c| c.starts_with(&format!("echo '{line}' >> ")));
        assert!(s.cfg.cli.vim_aliases && alias(&s), "on by default: {:?}", s.commands());
        assert_eq!(s.choices().first(), Some(&Choice::Toggle("vim")), "the aliases come first");
        s.flip("vim");
        assert!(!s.cfg.cli.vim_aliases && !alias(&s), "{:?}", s.commands());
    }

    #[test]
    fn the_look_starts_on_system_when_omarchy_is_found() {
        assert_eq!(Setup::new(None).cfg.theme.mode, Config::default().theme.mode);
        let o = Omarchy { name: "adventure".into(), palette: crate::theme::builtin("dark"), dir: "/themes/adventure".into() };
        let s = Setup::new(Some(&o));
        assert_eq!(s.cfg.theme.mode, "system", "System follows Omarchy");
        assert_eq!(s.omarchy_name.as_deref(), Some("adventure"));
    }

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
        s.terminal = true;
        s.dev_build = false;
        s.desktop_entry = true;
        s.exe = Some(PathBuf::from("/opt/nsvim/nsvim"));
        s.cfg.cli.default_app = true;
        let cmds = s.commands().join("\n");
        assert!(cmds.contains("export EDITOR=\"/opt/nsvim/nsvim --wait\" VISUAL=\"/opt/nsvim/nsvim --wait\""), "{cmds}");
        assert!(cmds.contains("git config --global core.editor \"/opt/nsvim/nsvim --wait\""), "{cmds}");
        assert!(cmds.contains("xdg-mime default nsvim.desktop text/plain text/x-rust"), "{cmds}");
        // Setup puts nothing on $PATH and adds no command of its own.
        assert!(!cmds.contains("ln -s") && !cmds.contains("export PATH") && !cmds.contains("alias nsvim"), "{cmds}");
        s.step = 4;
        assert_eq!(s.choices(), [Choice::Toggle("vim"), Choice::Toggle("env"), Choice::Toggle("git"), Choice::Toggle("default")]);
        s.desktop_entry = false;
        assert!(!s.commands().join("\n").contains("xdg-mime"), "macOS and Windows have no default-app step");
        assert!(!s.choices().contains(&Choice::Toggle("default")), "and no default-app choice");
    }

    /// Windows shells do not read the lines the terminal step writes, so Windows has no step.
    #[test]
    fn without_the_terminal_step_setup_goes_over_it_and_runs_nothing() {
        let mut a = App::new(Config::default(), PathBuf::from("/p"), true, None);
        let at = |a: &App| match &a.screen {
            Screen::Setup(s) => s.step,
            _ => 0,
        };
        let Screen::Setup(s) = &mut a.screen else { panic!("setup shows") };
        s.terminal = false;
        assert_eq!(s.steps(), ["Keys", "Mouse", "Look", "Ready"]);
        assert!(s.commands().is_empty(), "{:?}", s.commands());
        s.step = 3;
        a.setup_go(1);
        assert_eq!(at(&a), 5, "on from the look step to ready");
        a.setup_go(-1);
        assert_eq!(at(&a), 3, "back from ready to the look step");
        let Screen::Setup(s) = &mut a.screen else { panic!("setup shows") };
        s.step = 5;
        assert_eq!(s.choices().iter().filter(|c| matches!(c, Choice::Change(_))).count(), 4, "no terminal row");
    }

    /// `:Setup` starts from the config in use, and Cancel puts back the look it previewed.
    #[test]
    fn setup_again_starts_from_the_config_and_cancel_keeps_it() {
        let mut cfg = Config::default();
        cfg.keys.map = "classic".into();
        cfg.theme.mode = "light".into();
        let mut a = App::new(cfg, PathBuf::from("/p"), false, None);
        let mut tx = Tx::new();
        a.run_command("setup", &mut tx);
        let Screen::Setup(s) = &a.screen else { panic!("setup shows") };
        assert!(!s.vim() && s.cfg.theme.mode == "light" && s.prior.is_some(), "the current picks");
        a.setup_choose(Choice::Look(2), &mut tx);
        assert_eq!(a.cfg.theme.mode, "dark", "the window previews the pick");
        a.setup_key(KeyInput::plain(KeyCode::Esc), &mut tx);
        assert!(matches!(a.screen, Screen::Editor), "Esc on the first step leaves");
        assert_eq!(a.cfg.theme.mode, "light", "and puts the look back");
        assert_eq!(a.cfg.keys.map, "classic");
    }

    /// First launch has no editor to go back to: Esc and cancel stay in setup.
    #[test]
    fn first_launch_setup_cannot_be_cancelled() {
        let mut a = App::new(Config::default(), PathBuf::from("/p"), true, None);
        let mut tx = Tx::new();
        a.setup_key(KeyInput::plain(KeyCode::Esc), &mut tx);
        a.cancel_setup();
        assert!(matches!(a.screen, Screen::Setup(_)));
    }

    #[test]
    fn a_binary_in_a_cargo_target_folder_is_a_development_build() {
        for dev in [
            "/home/dev/nsvim/target/debug/nsvim",
            "/home/dev/nsvim/target/release/nsvim",
            "/home/dev/nsvim/target/quick/nsvim",
            "/home/dev/nsvim/target/x86_64-unknown-linux-gnu/release/nsvim",
            "/home/dev/nsvim/.nsh/rust/target/debug/nsh-app-nsvim-native-linux-host",
            "/home/dev/.cargo/targets/editor-7241c46d/debug/nsvim",
        ] {
            assert!(dev_path(std::path::Path::new(dev)), "{dev}");
        }
        for release in [
            "/home/dev/.local/share/nsvim/nsvim",
            "/Users/dev/Applications/nsvim.app/Contents/MacOS/nsvim",
            "/opt/target/nsvim",
            "/home/dev/debug/nsvim",
        ] {
            assert!(!dev_path(std::path::Path::new(release)), "{release}");
        }
    }

    #[test]
    fn a_path_with_spaces_or_quotes_stays_one_shell_word() {
        let mut s = Setup::new(None);
        s.exe = Some(PathBuf::from("/Users/dev/My Apps/nsvim.app/Contents/MacOS/nsvim"));
        assert_eq!(s.program(), "'/Users/dev/My Apps/nsvim.app/Contents/MacOS/nsvim'");
        assert_eq!(
            vim_aliases(&s.program()),
            "alias nvim=\"'/Users/dev/My Apps/nsvim.app/Contents/MacOS/nsvim'\" vim=\"'/Users/dev/My Apps/nsvim.app/Contents/MacOS/nsvim'\" vi=\"'/Users/dev/My Apps/nsvim.app/Contents/MacOS/nsvim'\""
        );
        s.exe = Some(PathBuf::from("/opt/it's $HOME/nsvim"));
        assert_eq!(s.program(), "'/opt/it'\\''s $HOME/nsvim'");
        assert_eq!(editor_env("'/a $b'"), "export EDITOR=\"'/a \\$b' --wait\" VISUAL=\"'/a \\$b' --wait\"");
    }

    /// The lines setup writes do what they say in a real shell: each alias runs the binary with
    /// the arguments, and `\nvim` skips the alias.
    #[cfg(unix)]
    #[test]
    fn the_alias_line_runs_the_binary_in_a_shell() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("my apps");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("nsvim");
        std::fs::write(&exe, "#!/bin/sh\necho \"nsvim $*\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("nvim"), "#!/bin/sh\necho neovim\n").unwrap();
        std::fs::set_permissions(real.join("nvim"), std::fs::Permissions::from_mode(0o755)).unwrap();
        let program = shell_word(&exe.display().to_string());
        let script = format!(
            "{}\n{}\nnvim src/main.rs:42\nvim .\nvi -V\n\\nvim\neval \"set -- $EDITOR\"\necho \"$#\"\n",
            vim_aliases(&program),
            editor_env(&program)
        );
        // bash expands aliases only when asked to outside an interactive shell.
        let out = std::process::Command::new("bash")
            .args(["-c", &format!("shopt -s expand_aliases\n{script}")])
            .env("PATH", format!("{}:/usr/bin:/bin", real.display()))
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(text, "nsvim src/main.rs:42\nnsvim .\nnsvim -V\nneovim\n2\n", "{}", String::from_utf8_lossy(&out.stderr));
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
