//! Colours: the built-in themes, the light / dark / system choice, and the theme Omarchy has
//! on the desktop right now.

use std::path::{Path, PathBuf};

use crate::color::{Color, hex};

/// Every colour the UI paints with. Names follow the design: `bg2` is the chrome behind the
/// text, `bg3` the cursor line and hovers, `ac` the accent and Normal mode, `ins` Insert, `vis`
/// Visual, `cmd` Command. `on` is text drawn on an accent fill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub bg: Color,
    pub bg2: Color,
    pub bg3: Color,
    pub bd: Color,
    pub fg: Color,
    pub fg2: Color,
    pub fg3: Color,
    pub ac: Color,
    pub ins: Color,
    pub vis: Color,
    pub cmd: Color,
    pub kw: Color,
    pub fun: Color,
    pub str_: Color,
    pub num: Color,
    pub ty: Color,
    pub mac: Color,
    pub com: Color,
    pub pun: Color,
    pub prop: Color,
    pub warn: Color,
    pub err: Color,
    pub sel: Color,
    pub on: Color,
    pub light: bool,
}

/// A built-in theme: its id in config.toml, its name in Settings and its colours as the design
/// writes them (bg bg2 bg3 bd fg fg2 fg3 ac ins vis cmd kw fn str num ty mac com pun prop warn
/// err sel on).
struct Builtin {
    id: &'static str,
    name: &'static str,
    light: bool,
    hex: [&'static str; 24],
}

const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "dark",
        name: "editor dark",
        light: false,
        hex: [
            "#0E1016", "#0A0C11", "#151925", "#232838", "#D6DAE4", "#8B93A6", "#7C8497", "#6C9EFF", "#A7D46F", "#C39BFF", "#FFA266",
            "#C39BFF", "#6C9EFF", "#A7D46F", "#FFA266", "#5ED3D3", "#F2777A", "#7C8497", "#8B93A6", "#E5C07B", "#E5C07B", "#F2777A",
            "#2A3150", "#0A0C11",
        ],
    },
    Builtin {
        id: "catppuccin",
        name: "Catppuccin Mocha",
        light: false,
        hex: [
            "#1E1E2E", "#181825", "#2A2B3C", "#313244", "#CDD6F4", "#A6ADC8", "#9095A9", "#89B4FA", "#A6E3A1", "#CBA6F7", "#FAB387",
            "#CBA6F7", "#89B4FA", "#A6E3A1", "#FAB387", "#F9E2AF", "#F38BA8", "#9095A9", "#9399B2", "#89DCEB", "#F9E2AF", "#F38BA8",
            "#45475A", "#181825",
        ],
    },
    Builtin {
        id: "tokyo-night",
        name: "Tokyo Night",
        light: false,
        hex: [
            "#1A1B26", "#16161E", "#232433", "#292E42", "#C0CAF5", "#A9B1D6", "#888EB0", "#7AA2F7", "#9ECE6A", "#BB9AF7", "#FF9E64",
            "#BB9AF7", "#7AA2F7", "#9ECE6A", "#FF9E64", "#2AC3DE", "#F7768E", "#888EB0", "#89DDFF", "#E0AF68", "#E0AF68", "#F7768E",
            "#283457", "#16161E",
        ],
    },
    Builtin {
        id: "gruvbox",
        name: "Gruvbox",
        light: false,
        hex: [
            "#282828", "#1D2021", "#32302F", "#3C3836", "#EBDBB2", "#BDAE93", "#A89984", "#83A598", "#B8BB26", "#D3869B", "#FE8019",
            "#FC6D5C", "#FABD2F", "#B8BB26", "#D3869B", "#8EC07C", "#FE8019", "#A4978B", "#A89984", "#83A598", "#FABD2F", "#FC6D5C",
            "#504945", "#1D2021",
        ],
    },
    Builtin {
        id: "kanagawa",
        name: "Kanagawa",
        light: false,
        hex: [
            "#1F1F28", "#16161D", "#2A2A37", "#363646", "#DCD7BA", "#C8C093", "#96958D", "#7E9CD8", "#98BB6C", "#A08CBF", "#FFA066",
            "#A08CBF", "#7E9CD8", "#98BB6C", "#D27E99", "#7AA89F", "#E6717E", "#96958D", "#9CABCA", "#E6C384", "#FF9E3B", "#EF6969",
            "#2D4F67", "#16161D",
        ],
    },
    Builtin {
        id: "rose-pine",
        name: "Rosé Pine",
        light: false,
        hex: [
            "#191724", "#151320", "#211F30", "#26233A", "#E0DEF4", "#908CAA", "#8B88A1", "#C4A7E7", "#9CCFD8", "#C4A7E7", "#F6C177",
            "#4195B8", "#EBBCBA", "#F6C177", "#EBBCBA", "#9CCFD8", "#EB6F92", "#8B88A1", "#908CAA", "#C4A7E7", "#F6C177", "#EB6F92",
            "#403D52", "#151320",
        ],
    },
    Builtin {
        id: "one-dark",
        name: "One Dark",
        light: false,
        hex: [
            "#282C34", "#21252B", "#2C313C", "#3E4451", "#ABB2BF", "#9DA5B4", "#959BA8", "#61AFEF", "#98C379", "#C980DF", "#D19A66",
            "#C980DF", "#61AFEF", "#98C379", "#D19A66", "#E5C07B", "#56B6C2", "#959BA8", "#ABB2BF", "#E47D85", "#E5C07B", "#E47D85",
            "#3E4451", "#21252B",
        ],
    },
    Builtin {
        id: "everforest",
        name: "Everforest",
        light: false,
        hex: [
            "#2D353B", "#232A2E", "#343F44", "#3D484D", "#D3C6AA", "#A2AEA5", "#A3ACA6", "#7FBBB3", "#A7C080", "#D699B6", "#E69875",
            "#EA9395", "#A7C080", "#DBBC7F", "#D699B6", "#83C092", "#E69875", "#A3ACA6", "#A2AEA5", "#7FBBB3", "#DBBC7F", "#EA9395",
            "#475258", "#232A2E",
        ],
    },
    Builtin {
        id: "nord",
        name: "Nord",
        light: false,
        hex: [
            "#2E3440", "#272C36", "#3B4252", "#434C5E", "#ECEFF4", "#D8DEE9", "#A8B0C0", "#88C0D0", "#A3BE8C", "#C4A7BF", "#DBA493",
            "#99B3CD", "#88C0D0", "#A3BE8C", "#C4A7BF", "#8FBCBB", "#D9A2A8", "#A8B0C0", "#99B3CD", "#EBCB8B", "#EBCB8B", "#D9A2A8",
            "#434C5E", "#272C36",
        ],
    },
    Builtin {
        id: "light",
        name: "editor light",
        light: true,
        hex: [
            "#FAFAF7", "#F0F0EB", "#ECEDF2", "#D9DAE0", "#1F2330", "#5B6273", "#626877", "#2F5FD0", "#39761D", "#7A3FC8", "#AB5019",
            "#7A3FC8", "#2F5FD0", "#39761D", "#AB5019", "#0E7575", "#B83A3E", "#626877", "#5B6273", "#8A6200", "#8A6200", "#B83A3E",
            "#D3DDF8", "#FFFFFF",
        ],
    },
    Builtin {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        light: true,
        hex: [
            "#EFF1F5", "#E6E9EF", "#E2E5ED", "#CCD0DA", "#4C4F69", "#616377", "#606376", "#0A55EA", "#2D701E", "#8230EE", "#AC4001",
            "#8230EE", "#0A55EA", "#2D701E", "#AC4001", "#8E5A12", "#C80E36", "#606376", "#616476", "#127076", "#8E5A12", "#C80E36",
            "#CCD0DA", "#FFFFFF",
        ],
    },
    Builtin {
        id: "tokyo-night-day",
        name: "Tokyo Night Day",
        light: true,
        hex: [
            "#E1E2E7", "#D5D6DB", "#D7D9E3", "#C4C8DA", "#3156AB", "#485891", "#4F5884", "#1359B7", "#49602F", "#761BEC", "#8D4900",
            "#761BEC", "#1359B7", "#49602F", "#8D4900", "#006283", "#B6093B", "#4F5884", "#006279", "#705631", "#705631", "#B6093B",
            "#B7C1E3", "#FFFFFF",
        ],
    },
    Builtin {
        id: "gruvbox-light",
        name: "Gruvbox Light",
        light: true,
        hex: [
            "#FBF1C7", "#F2E5BC", "#F2E5BC", "#D5C4A1", "#3C3836", "#665C54", "#6E6259", "#076678", "#6B670C", "#8F3F71", "#AF3A03",
            "#9D0006", "#6B670C", "#6B670C", "#8F3F71", "#8C5B0F", "#3B6E4F", "#6E6256", "#6E6259", "#076678", "#8C5B0F", "#9D0006",
            "#D5C4A1", "#FFFFFF",
        ],
    },
    Builtin {
        id: "rose-pine-dawn",
        name: "Rosé Pine Dawn",
        light: true,
        hex: [
            "#FAF4ED", "#F2E9E1", "#F4EDE8", "#DFDAD9", "#575279", "#686480", "#6A6478", "#286983", "#416F77", "#765E92", "#945C0F",
            "#286983", "#B63E39", "#945C0F", "#A14D65", "#416F77", "#A14D65", "#6A6478", "#686480", "#765E92", "#945C0F", "#A14D65",
            "#DFDAD9", "#FFFFFF",
        ],
    },
    Builtin {
        id: "one-light",
        name: "One Light",
        light: true,
        hex: [
            "#FAFAFA", "#F0F0F1", "#EEEEEF", "#DBDBDC", "#383A42", "#676A74", "#676870", "#1A5DEF", "#3A7539", "#A626A4", "#8E6101",
            "#A626A4", "#1A5DEF", "#3A7539", "#8E6101", "#8E6101", "#0172A3", "#676870", "#383A42", "#C82C1E", "#8E6101", "#C82C1E",
            "#E5E5E6", "#FFFFFF",
        ],
    },
    Builtin {
        id: "flexoki-light",
        name: "Flexoki Light",
        light: true,
        hex: [
            "#FFFCF0", "#F2F0E5", "#ECE9DD", "#DAD8CE", "#100F0F", "#676662", "#686662", "#205EA6", "#576D09", "#5E409D", "#AA4A13",
            "#5E409D", "#AA4A13", "#20736C", "#5E409D", "#7F6001", "#AF3029", "#686662", "#676662", "#205EA6", "#7F6001", "#AF3029",
            "#DAD8CE", "#FFFFFF",
        ],
    },
];

/// The dark themes Settings offers for `theme.dark`, in its order.
pub const DARK_THEMES: &[&str] = &["dark", "catppuccin", "tokyo-night", "gruvbox", "kanagawa", "rose-pine", "one-dark"];
/// The light themes Settings offers for `theme.light`.
pub const LIGHT_THEMES: &[&str] = &["light", "catppuccin-latte", "tokyo-night-day", "gruvbox-light", "rose-pine-dawn", "one-light"];

fn c(s: &str) -> Color {
    hex(s).unwrap_or(Color::BLACK)
}

fn from_builtin(b: &Builtin) -> Palette {
    let h = b.hex.map(c);
    Palette {
        bg: h[0],
        bg2: h[1],
        bg3: h[2],
        bd: h[3],
        fg: h[4],
        fg2: h[5],
        fg3: h[6],
        ac: h[7],
        ins: h[8],
        vis: h[9],
        cmd: h[10],
        kw: h[11],
        fun: h[12],
        str_: h[13],
        num: h[14],
        ty: h[15],
        mac: h[16],
        com: h[17],
        pun: h[18],
        prop: h[19],
        warn: h[20],
        err: h[21],
        sel: h[22],
        on: h[23],
        light: b.light,
    }
}

/// The built-in theme `id`, or `dark` when there is none by that id.
pub fn builtin(id: &str) -> Palette {
    from_builtin(BUILTINS.iter().find(|b| b.id == id).unwrap_or(&BUILTINS[0]))
}

/// A built-in theme's display name (`Tokyo Night`), or the id itself.
pub fn name(id: &str) -> &str {
    BUILTINS.iter().find(|b| b.id == id).map_or(id, |b| b.name)
}

/// The five swatches Settings shows for a theme: background, accent, Insert, Visual, Command.
pub fn swatches(id: &str) -> [Color; 5] {
    let p = builtin(id);
    [p.bg, p.ac, p.ins, p.vis, p.cmd]
}

/// The theme on the desktop, as Omarchy writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Omarchy {
    /// The theme's name (`tokyo-night`), from `theme.name` or the folder.
    pub name: String,
    pub palette: Palette,
    /// The folder it was read from.
    pub dir: PathBuf,
}

/// Where Omarchy keeps the active theme: `~/.local/state/omarchy/current/theme` in newer
/// releases, `~/.config/omarchy/current/theme` before that.
pub fn omarchy_theme_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    [".local/state/omarchy/current/theme", ".config/omarchy/current/theme"].iter().map(|p| home.join(p)).find(|p| p.is_dir())
}

/// Reads the active Omarchy theme, if Omarchy is installed. Does file I/O: call it off the UI
/// thread or at startup.
pub fn read_omarchy() -> Option<Omarchy> {
    let dir = omarchy_theme_dir()?;
    let name = std::fs::read_to_string(dir.with_file_name("theme.name"))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let palette = read_colors_toml(&dir).or_else(|| read_alacritty(&dir))?;
    Some(Omarchy { name, palette, dir })
}

/// Omarchy's `colors.toml`: named colours plus `mode = "light" | "dark"`.
fn read_colors_toml(dir: &Path) -> Option<Palette> {
    let src = std::fs::read_to_string(dir.join("colors.toml")).ok()?;
    let table: toml::Table = src.parse().ok()?;
    let get = |k: &str| table.get(k).and_then(|v| v.as_str()).and_then(hex);
    let bg = get("background")?;
    let fg = get("foreground")?;
    let light = table.get("mode").and_then(|v| v.as_str()) == Some("light") || dir.join("light.mode").exists();
    Some(derive(bg, fg, light, |k| get(k)))
}

/// Older themes only ship terminal colours: `alacritty.toml`'s `[colors.primary]` and
/// `[colors.normal]`.
fn read_alacritty(dir: &Path) -> Option<Palette> {
    let src = std::fs::read_to_string(dir.join("alacritty.toml")).ok()?;
    let table: toml::Table = src.parse().ok()?;
    let colors = table.get("colors")?.as_table()?;
    let section = |s: &str, k: &str| colors.get(s)?.as_table()?.get(k)?.as_str().and_then(|v| hex(&v.replace("0x", "#")));
    let bg = section("primary", "background")?;
    let fg = section("primary", "foreground")?;
    let light = dir.join("light.mode").exists();
    Some(derive(bg, fg, light, |k| match k {
        "accent" | "blue" => section("normal", "blue"),
        "green" => section("normal", "green"),
        "magenta" => section("normal", "magenta"),
        "orange" | "yellow" => section("normal", "yellow"),
        "cyan" => section("normal", "cyan"),
        "red" => section("normal", "red"),
        _ => None,
    }))
}

/// A full palette from a background, a foreground and whatever named colours the theme has.
fn derive(bg: Color, fg: Color, light: bool, get: impl Fn(&str) -> Option<Color>) -> Palette {
    let base = if light { builtin("light") } else { builtin("dark") };
    let pick = |keys: &[&str], fallback: Color| keys.iter().find_map(|k| get(k)).unwrap_or(fallback);
    let ac = pick(&["accent", "blue"], base.ac);
    let toward = |t: f32| bg.mix(fg, t);
    Palette {
        bg,
        bg2: pick(&["dark_background", "darker_background"], if light { bg.mix(Color::BLACK, 0.04) } else { bg.mix(Color::BLACK, 0.25) }),
        bg3: pick(&["lighter_background"], toward(0.06)),
        bd: toward(0.14),
        fg,
        fg2: pick(&["light_foreground"], toward(0.62)),
        fg3: pick(&["muted", "dark_foreground"], toward(0.48)),
        ac,
        ins: pick(&["green"], base.ins),
        vis: pick(&["magenta"], base.vis),
        cmd: pick(&["orange", "yellow"], base.cmd),
        kw: pick(&["magenta"], base.kw),
        fun: pick(&["blue", "accent"], base.fun),
        str_: pick(&["green"], base.str_),
        num: pick(&["orange", "yellow"], base.num),
        ty: pick(&["cyan"], base.ty),
        mac: pick(&["red"], base.mac),
        com: pick(&["muted", "dark_foreground"], toward(0.48)),
        pun: pick(&["light_foreground"], toward(0.62)),
        prop: pick(&["yellow"], base.prop),
        warn: pick(&["yellow"], base.warn),
        err: pick(&["red"], base.err),
        sel: pick(&["selection"], toward(0.18)),
        on: if light { Color::WHITE } else { bg.mix(Color::BLACK, 0.3) },
        light,
    }
}

/// What `theme` in config.toml asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    Light,
    #[default]
    Dark,
    System,
    Omarchy,
}

impl Source {
    pub fn parse(s: &str) -> Source {
        match s {
            "light" => Source::Light,
            "system" => Source::System,
            "omarchy" => Source::Omarchy,
            _ => Source::Dark,
        }
    }
}

/// The palette to paint with now, and the id of the theme it came from.
pub fn resolve(source: Source, dark_id: &str, light_id: &str, system_dark: bool, omarchy: Option<&Omarchy>) -> (Palette, String) {
    match source {
        Source::Light => (builtin(light_id), light_id.to_string()),
        Source::Dark => (builtin(dark_id), dark_id.to_string()),
        Source::System if system_dark => (builtin(dark_id), dark_id.to_string()),
        Source::System => (builtin(light_id), light_id.to_string()),
        Source::Omarchy => match omarchy {
            Some(o) => (o.palette, o.name.clone()),
            None => (builtin(dark_id), dark_id.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_parses() {
        for b in BUILTINS {
            for h in b.hex {
                assert!(hex(h).is_some(), "{} has a bad colour {h}", b.id);
            }
        }
    }

    #[test]
    fn omarchy_colors_toml_maps_onto_a_palette() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("colors.toml"),
            "mode = \"dark\"\naccent = \"#f0a640\"\nbackground = \"#0d0d0c\"\nforeground = \"#cfccc4\"\ngreen = \"#a3b07e\"\n",
        )
        .unwrap();
        let p = read_colors_toml(dir.path()).unwrap();
        assert_eq!(p.ac, hex("#f0a640").unwrap());
        assert_eq!(p.ins, hex("#a3b07e").unwrap());
        assert!(!p.light);
    }
}
