//! The five steps of the first-launch setup. Each step is made again from the setup's model
//! when it changes. A press picks a choice through `App::setup_choose`, as Enter does.

use std::path::Path;

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_base::{ButtonElement, button};
use nsh_core::prelude::*;

use crate::app::setup::{Choice, FONTS, LOOKS, SIZES};
use crate::color::Color;
use crate::store::Session;
use crate::theme::Palette;
use crate::ui::settings_controls::{act, choice, kbd, pill, titled, toggle};
use crate::ui::setup_view::Model;
use crate::ui::style::{color, swatch};

pub fn body(s: Session, keys: ElementRef, m: &Model) -> AnyElement {
    let k = Keys { s, keys };
    match m.step {
        1 => step_keys(k, m),
        2 => step_mouse(k, m),
        3 => step_look(k, m),
        4 => step_shell(k, m),
        _ => step_ready(k, m),
    }
    .test_id(&format!("setup-step-{}", m.step))
    .into_element()
}

/// What a press needs: the session and the view that takes the keys back after it.
#[derive(Clone, Copy)]
struct Keys {
    s: Session,
    keys: ElementRef,
}

impl Keys {
    fn pick(self, c: Choice) -> impl Fn() + 'static {
        act(self.s, self.keys, move |a, tx| a.setup_choose(c, tx))
    }
}

fn heading(t: &str, sub: &str) -> impl IntoElement {
    view()
        .gap(px(6.0))
        .child(text(t.to_string()).text_2xl().font_weight(FontWeight::BOLD).text_color(move || color("foreground")))
        .child(text(sub.to_string()).text_sm().text_color(move || color("muted-foreground")))
}

/// A card that picks one option: a tag over its border, a title, a `( )` or `(•)` mark, a
/// description and a body.
#[allow(clippy::too_many_arguments)]
fn card(k: Keys, c: Choice, tag: &str, title: &str, desc: &str, on: bool, focus: bool, body: impl IntoElement) -> ButtonElement {
    let border = if on {
        "primary"
    } else if focus {
        "text-faint"
    } else {
        "border"
    };
    let mark = if on { "primary" } else { "text-faint" };
    button()
        .role(AccessibilityRole::Radio)
        .selected(on)
        .accessibility_label(SharedString::from(title.to_string()))
        .on_press(k.pick(c))
        .flex_1()
        .min_w(px(260.0))
        .gap(px(12.0))
        .pt(px(18.0))
        .pb(px(14.0))
        .px(px(18.0))
        .rounded(px(6.0))
        .bg(move || color("background"))
        .hover(|st| st.bg(move || color("surface-raised")))
        // The border is a child under the tag, so the tag hides the line under it.
        .child(view().absolute().inset(px(0.0)).rounded(px(6.0)).border(px(if on { 2.0 } else { 1.0 })).border_color(move || color(border)))
        .child(
            view()
                .absolute()
                .left(px(12.0))
                .top(px(-9.0))
                .z_index(1)
                .px(px(5.0))
                .bg(move || color("background"))
                .child(text(tag.to_string()).text_xs().text_color(move || color(if on { "primary" } else { "muted-foreground" }))),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .child(text(title.to_string()).text_base().font_weight(FontWeight::BOLD).flex_1().text_color(move || color("foreground")))
                .child(text(if on { "(•)" } else { "( )" }).text_sm().text_color(move || color(mark))),
        )
        // The description takes the free height, so the bodies of the cards in a row line up.
        .child(text(desc.to_string()).text_xs().flex_1().text_color(move || color("muted-foreground")))
        .child(body)
}

/// A key and what it does, in a card.
fn krow(name: &str, v: &str) -> impl IntoElement {
    view()
        .flex_row()
        .h(px(24.0))
        .w_full()
        .items_center()
        .border_t_1()
        .border_color(move || color("border"))
        .child(text(name.to_string()).flex_1().text_xs().text_color(move || color("muted-foreground")))
        .child(text(v.to_string()).text_xs().font_weight(FontWeight::BOLD).text_color(move || color("foreground")))
}

/// A switch with a label and a line under it. `focus` marks the keyboard's place.
fn switch_row(k: Keys, c: Choice, label: &str, sub: &str, on: bool, focus: bool) -> impl IntoElement {
    let mut words = view().flex_1().gap(px(2.0)).child(text(label.to_string()).text_sm().text_color(move || color("foreground")));
    if !sub.is_empty() {
        words = words.child(text(sub.to_string()).text_xs().text_color(move || color("muted-foreground")));
    }
    view()
        .flex_row()
        .w_full()
        .p(px(8.0))
        .gap(px(12.0))
        .items_center()
        .rounded_md()
        .bg(move || color(if focus { "surface-raised" } else { "background" }))
        .child(toggle(label, move || on, k.pick(c)))
        .child(words)
}

fn box_of(title: &str) -> View {
    titled(title.to_string()).w_full().pt(px(14.0)).pb(px(8.0)).px(px(8.0))
}

fn step_keys(k: Keys, m: &Model) -> View {
    let vim = m.vim();
    let md = m.mod_key();
    let leader = if m.on("leader") { "SPC" } else { "\\" };
    let vim_body = view()
        .gap(px(12.0))
        .child(
            view()
                .flex_row()
                .gap(px(6.0))
                .child(pill("NORMAL", "primary", "primary-foreground"))
                .child(pill("INSERT", "mode-insert", "primary-foreground"))
                .child(pill("VISUAL", "mode-visual", "primary-foreground")),
        )
        .child(
            view()
                .child(krow("save", ":w"))
                .child(krow("find file", &format!("{leader} f")))
                .child(krow("search", "/"))
                .child(krow("comment", "gcc")),
        );
    let classic_body = view()
        .gap(px(12.0))
        .child(
            view()
                .flex_row()
                .gap(px(6.0))
                .child(pill(md, "surface-raised", "foreground"))
                .child(pill(if m.mac { "⌥" } else { "Alt" }, "surface-raised", "foreground"))
                .child(pill("⇧", "surface-raised", "foreground")),
        )
        .child(
            view()
                .child(krow("save", &format!("{md} S")))
                .child(krow("find file", &format!("{md} P")))
                .child(krow("search", &format!("{md} F")))
                .child(krow("comment", &format!("{md} /"))),
        );
    let toggles: [(&'static str, &str, &str); 2] = if vim {
        [
            ("leader", "Use Space as the leader key", "Otherwise backslash, as in stock Neovim."),
            ("import", "Import keymaps from ~/.config/nvim", "Reads your leader. Plugins are skipped."),
        ]
    } else {
        [
            ("multi", "Add cursors with Alt-click", "Every cursor types at once."),
            ("autoclose", "Auto-close brackets and quotes", "Typing ( also types )."),
        ]
    };
    let mut opts = box_of("options");
    for (key, label, sub) in toggles {
        let c = Choice::Toggle(key);
        opts = opts.child(switch_row(k, c, label, sub, m.on(key), m.focused(c)));
    }
    let platform = |mac: bool, label: &str| {
        let c = Choice::Platform(mac);
        let on = m.mac == mac;
        let focus = m.focused(c);
        choice(&format!("platform:{label}"), label.to_string(), move || on, move || focus, k.pick(c))
    };
    view()
        .gap(px(22.0))
        .child(
            view()
                .flex_row()
                .items_end()
                .flex_wrap()
                .gap(px(20.0))
                .child(heading("How do you want to edit?", "Switch later with :set keymap."))
                .child(view().flex_1())
                .child(
                    view()
                        .flex_row()
                        .gap(px(6.0))
                        .items_center()
                        .child(text("keys shown for").text_xs().text_color(move || color("muted-foreground")))
                        .child(platform(true, "macOS"))
                        .child(platform(false, "linux / windows")),
                ),
        )
        .child(
            view()
                .flex_row()
                .gap(px(16.0))
                .flex_wrap()
                .items_start()
                .child(card(
                    k,
                    Choice::Keymap(true),
                    "neovim",
                    "Neovim motions",
                    "Modal: move in Normal, type in Insert. Your muscle memory works as is.",
                    vim,
                    m.focused(Choice::Keymap(true)),
                    vim_body,
                ))
                .child(card(
                    k,
                    Choice::Keymap(false),
                    "classic",
                    "Classic shortcuts",
                    "The keys from most editors. No modes. Start typing straight away.",
                    !vim,
                    m.focused(Choice::Keymap(false)),
                    classic_body,
                )),
        )
        .child(opts)
}

fn step_mouse(k: Keys, m: &Model) -> View {
    let on = m.cfg.mouse.enabled;
    let vim = m.vim();
    let on_body = view().rounded_md().border_1().border_color(move || color("border")).p(px(8.0)).child(
        text("let mut pos = from;\nfor _ in 0..count.max(1) {\n    pos = buf.next(pos);").text_xs().text_color(move || color("foreground")),
    );
    let off_body =
        view().child(krow("move", if vim { "h j k l" } else { "← ↑ ↓ →" })).child(krow("select", if vim { "v" } else { "⇧ arrows" }));
    let mut out =
        view().gap(px(22.0)).child(heading("Use the mouse?", "nsvim is keyboard-first. This only changes the text area and panes.")).child(
            view()
                .flex_row()
                .gap(px(16.0))
                .flex_wrap()
                .items_start()
                .child(card(
                    k,
                    Choice::Mouse(true),
                    "mouse=a",
                    "Mouse on",
                    "Click to place the cursor, drag to select, scroll, resize the file tree.",
                    on,
                    m.focused(Choice::Mouse(true)),
                    on_body,
                ))
                .child(card(
                    k,
                    Choice::Mouse(false),
                    "mouse=",
                    "Keyboard only",
                    "Clicks inside the editor do nothing, so a stray click never moves the cursor. The wheel still scrolls.",
                    !on,
                    m.focused(Choice::Mouse(false)),
                    off_body,
                )),
        );
    if on {
        let mut b = box_of("the mouse can");
        let drag = if vim { "Drag selects and enters Visual mode" } else { "Drag selects text" };
        for (key, label) in [
            ("click", "Click places the cursor"),
            ("drag", drag),
            ("double", "Double-click selects a word, triple-click a line"),
            ("right", "Right-click opens a context menu"),
        ] {
            let c = Choice::Toggle(key);
            b = b.child(switch_row(k, c, label, "", m.on(key), m.focused(c)));
        }
        out = out.child(b);
    }
    out
}

fn step_look(k: Keys, m: &Model) -> View {
    // On Omarchy, System follows the desktop theme. Elsewhere it follows the OS.
    let system_desc = match &m.omarchy_name {
        Some(name) => format!("Follows the Omarchy theme, now {name}."),
        None => "Light or dark with your OS.".to_string(),
    };
    let descs = ["Paper white, dark ink.", "The default. Low glare at night.", system_desc.as_str()];
    let titles = ["Light", "Dark", "System"];
    // The cards in a row have the same height, as tall as the tallest.
    let mut cards = view().flex_row().gap(px(12.0)).flex_wrap().items_stretch();
    for (i, id) in LOOKS.iter().enumerate() {
        let on = m.cfg.theme.mode == *id;
        let comment = match *id {
            "system" => m.omarchy_name.clone().unwrap_or_else(|| "follows the OS".into()),
            other => other.to_string(),
        };
        let lp = m.looks.get(i).copied().unwrap_or_else(|| crate::theme::builtin("dark"));
        let c = Choice::Look(i);
        cards = cards.child(card(k, c, id, titles[i], descs[i], on, m.focused(c), sample(&lp, &comment)).min_w(px(190.0)));
    }
    let mut fonts = view().flex_row().gap(px(6.0)).flex_wrap();
    for (i, f) in FONTS.iter().enumerate() {
        let c = Choice::Font(i);
        let (on, focus) = (m.cfg.font.family == *f, m.focused(c));
        fonts = fonts.child(choice(&format!("font:{f}"), *f, move || on, move || focus, k.pick(c)));
    }
    let mut sizes = view().flex_row().gap(px(6.0));
    for (i, z) in SIZES.iter().enumerate() {
        let c = Choice::Size(i);
        let (on, focus) = (m.cfg.font.size == *z, m.focused(c));
        sizes = sizes.child(choice(&format!("size:{z}"), format!("{z} pt"), move || on, move || focus, k.pick(c)));
    }
    let family = crate::geometry::family(&m.cfg.font.family);
    let size = m.cfg.font.size;
    let sample_text =
        |t: &str, token: &'static str| text(t.to_string()).font_family(family).text_size(px(size)).text_color(move || color(token));
    let font_box = titled("font")
        .w_full()
        .pt(px(16.0))
        .pb(px(12.0))
        .px(px(16.0))
        .gap(px(12.0))
        .child(view().flex_row().gap(px(16.0)).items_center().child(label("family")).child(fonts))
        .child(view().flex_row().gap(px(16.0)).items_center().child(label("size")).child(sizes))
        .child(
            view()
                .rounded_md()
                .p(px(8.0))
                .border_1()
                .border_color(move || color("border"))
                .child(sample_text("/// The quick brown fox -> jumps != over", "code-comment"))
                .child(sample_text("pub fn word_forward(buf: &Buffer, from: Pos) -> Target {", "foreground")),
        );
    view()
        .gap(px(22.0))
        .child(heading("Pick a look", "This window follows your pick. Colors, fonts and spacing are all in settings later."))
        .child(cards)
        .child(font_box)
}

fn label(t: &str) -> impl IntoElement {
    text(t.to_string()).w(px(64.0)).text_xs().text_color(move || color("muted-foreground"))
}

/// A few lines of Rust in a look's own colours. The colours are data: the look is not the
/// theme that shows now.
fn sample(p: &Palette, comment: &str) -> impl IntoElement {
    let src = format!("fn class(c: char) {{\n    // {comment}\n    \"word\"\n}}");
    let lines: Vec<&str> = src.lines().collect();
    let runs = highlight::runs(&Buffer::from_str(&src), Some(Path::new("look.rs")), 0, lines.len());
    let mut out = view().rounded_md().p(px(8.0)).bg(swatch(p.bg)).border_1().border_color(swatch(p.bd));
    for (i, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut t = text("").text_xs().number_of_lines(1);
        let mut col = 0;
        for (r, kind) in runs.get(i).cloned().unwrap_or_default() {
            if r.start > col && r.start <= chars.len() {
                t = t.child(text(chars[col..r.start].iter().collect::<String>()).text_color(swatch(p.fg)));
            }
            let end = r.end.min(chars.len());
            let a = r.start.max(col);
            if a < end {
                t = t.child(text(chars[a..end].iter().collect::<String>()).text_color(swatch(kind_color(p, kind))));
            }
            col = col.max(end);
        }
        if col < chars.len() {
            t = t.child(text(chars[col..].iter().collect::<String>()).text_color(swatch(p.fg)));
        }
        out = out.child(t);
    }
    out
}

fn kind_color(p: &Palette, k: Kind) -> Color {
    match k {
        Kind::Keyword => p.kw,
        Kind::Function => p.fun,
        Kind::Type => p.ty,
        Kind::String => p.str_,
        Kind::Number => p.num,
        Kind::Comment => p.com,
        Kind::Macro => p.mac,
        Kind::Property => p.prop,
        Kind::Punct => p.pun,
        Kind::Text => p.fg,
    }
}

fn step_shell(k: Keys, m: &Model) -> View {
    let out = view().gap(px(22.0)).child(heading("Use nsvim from the terminal", "Optional. Everything here can be undone by hand later."));
    let cli = &m.cfg.cli;
    // The command the shell runs: vi and vim with the aliases, else nsvim.
    let cmd = if m.on("vim") { "vi" } else { cli.name.as_str() };
    let mut integ = titled("integrations").flex_1().min_w(px(360.0)).pt(px(14.0)).pb(px(8.0)).px(px(8.0));
    for (key, label, sub) in [
        ("vim", "Open nsvim with vi and vim".to_string(), format!("Shell aliases. \\vim still runs Vim. Off: type {}", cli.name)),
        ("path", format!("Add the {} command", cli.name), format!("Open anything with {cmd} . or {cmd} file.rs")),
        ("env", "Set as $EDITOR and $VISUAL".into(), "For tools that open an editor, like crontab -e".into()),
        ("git", "Use for git commit messages".into(), "git waits until you close the buffer".into()),
        ("default", "Make nsvim the default for text files".into(), "A double-click opens code and text files".into()),
    ] {
        if key == "default" && m.mac {
            continue;
        }
        let c = Choice::Toggle(key);
        integ = integ.child(switch_row(k, c, &label, &sub, m.on(key), m.focused(c)));
        if let Some(t) = m.taken.as_ref().filter(|_| key == "path" && cli.path) {
            integ = integ.child(taken_row(k, m, t));
        }
    }
    let mut runs =
        titled("what runs").flex_1().min_w(px(360.0)).pt(px(16.0)).pb(px(14.0)).px(px(16.0)).gap(px(4.0)).test_id("setup-commands");
    if m.commands.is_empty() {
        runs = runs.child(text("nothing. nsvim stays inside its window").text_xs().text_color(move || color("muted-foreground")));
    }
    for cmd in &m.commands {
        // A `#` line says what a step writes, as a comment in a shell script does.
        let note = cmd.starts_with("# ");
        runs = runs.child(
            view()
                .flex_row()
                .w_full()
                .items_start()
                .child(text(if note { "" } else { "$ " }).text_xs().text_color(move || color("text-faint")))
                .child(
                    text(cmd.clone())
                        .flex_1()
                        .min_w(px(0.0))
                        .text_xs()
                        .text_color(move || color(if note { "muted-foreground" } else { "foreground" })),
                ),
        );
    }
    out.child(view().flex_row().gap(px(16.0)).flex_wrap().items_start().child(integ).child(runs))
}

/// Another program has the command's name on `$PATH`. The user can put nsvim's link first.
fn taken_row(k: Keys, m: &Model, taken: &str) -> impl IntoElement {
    let name = &m.cfg.cli.name;
    let other = taken.split(',').next().unwrap_or("").to_string();
    let shadow = format!("Put nsvim's {name} first on $PATH");
    view()
        .pl(px(8.0))
        .gap(px(4.0))
        .test_id("command-taken")
        .child(text(format!("{name} is already {other}.")).text_xs().text_color(move || color("warning")))
        .child(switch_row(
            k,
            Choice::Shadow,
            &shadow,
            "Without it, the aliases and $EDITOR use the full path",
            m.shadow,
            m.focused(Choice::Shadow),
        ))
}

fn step_ready(k: Keys, m: &Model) -> View {
    let vim = m.vim();
    let md = m.mod_key();
    let cfg = &m.cfg;
    let leader = if cfg.keys.leader == "space" { "Space" } else { "backslash" };
    let shell: Vec<&str> =
        [("path", "command"), ("vim", "vi and vim"), ("env", "$EDITOR"), ("git", "git editor"), ("default", "default app")]
            .iter()
            .filter(|(key, _)| m.on(key) && (*key != "path" || m.installable))
            .map(|(_, v)| *v)
            .collect();
    let rows = [
        ("keys", if vim { format!("Neovim motions, {leader} leader") } else { format!("Classic shortcuts ({md})") }),
        ("mouse", if cfg.mouse.enabled { "on".into() } else { "keyboard only".into() }),
        (
            "theme",
            if let Some(name) = m.omarchy_name.clone().filter(|_| cfg.theme.mode == "system") {
                format!("Omarchy ({name} right now)")
            } else {
                cfg.theme.mode.clone()
            },
        ),
        ("font", format!("{}, {} pt", cfg.font.family, cfg.font.size)),
        ("terminal", if shell.is_empty() { "none".into() } else { shell.join(", ") }),
    ];
    let mut summary = titled("your setup").flex_1().min_w(px(420.0)).pt(px(12.0)).pb(px(6.0)).px(px(16.0)).test_id("setup-summary");
    for (i, (name, v)) in rows.into_iter().enumerate() {
        let c = Choice::Change(i);
        let focus = m.focused(c);
        summary = summary.child(
            view()
                .flex_row()
                .min_h(px(34.0))
                .w_full()
                .gap(px(16.0))
                .items_center()
                .border_b_1()
                .border_color(move || color("border"))
                .child(text(name).w(px(84.0)).text_sm().text_color(move || color("muted-foreground")))
                .child(text(v).flex_1().text_sm().text_color(move || color("foreground")))
                .child(
                    button()
                        .accessibility_label(SharedString::from(format!("change {name}")))
                        .on_press(k.pick(c))
                        .px(px(4.0))
                        .rounded(px(3.0))
                        .bg(move || color(if focus { "surface-raised" } else { "background" }))
                        .hover(|st| st.opacity(0.8))
                        .child(text("change").text_xs().text_color(move || color("primary"))),
                ),
        );
    }
    let lk = if cfg.keys.leader == "space" { "SPC" } else { "\\" };
    let tips: Vec<(&str, Vec<String>)> = if vim {
        vec![
            ("Find a file", vec![lk.into(), "f".into()]),
            ("Every key, in a popup", vec![lk.into()]),
            ("Settings", vec![":Settings".into()]),
        ]
    } else {
        vec![
            ("Find a file", vec![md.into(), "P".into()]),
            ("Command palette", vec![md.into(), "⇧".into(), "P".into()]),
            ("Settings", vec![md.into(), ",".into()]),
        ]
    };
    let mut tips_box = titled("try first").flex_1().min_w(px(300.0)).pt(px(12.0)).pb(px(6.0)).px(px(16.0));
    for (t, ks) in tips {
        let mut kr = view().flex_row().gap(px(4.0));
        for key in ks {
            kr = kr.child(kbd(key));
        }
        tips_box = tips_box.child(
            view()
                .flex_row()
                .min_h(px(34.0))
                .w_full()
                .items_center()
                .border_b_1()
                .border_color(move || color("border"))
                .child(text(t).flex_1().text_sm().text_color(move || color("foreground")))
                .child(kr),
        );
    }
    view()
        .gap(px(22.0))
        .child(heading("Ready", "Saved to ~/.config/nsvim/config.toml when you open nsvim."))
        .child(view().flex_row().gap(px(16.0)).flex_wrap().items_start().child(summary).child(tips_box))
}
