//! First-launch setup: keys, mouse, look, terminal, ready. Each step is cards and switches.
//! The status line shows where you are and moves on. The keys work as the hint line says.

use nsh_base::button;
use nsh_core::prelude::*;

use crate::app::setup::{Choice, LOOKS, STEPS, Setup};
use crate::app::{App, Screen};
use crate::config::Config;
use crate::store::Session;
use crate::theme::{self, Palette};
use crate::ui::settings_controls::act;
use crate::ui::setup_steps;
use crate::ui::style::color;

/// What the setup screen shows: a copy of the setup's state, so a memo can compare it.
#[derive(Clone, PartialEq)]
pub struct Model {
    pub step: usize,
    pub focus: Option<Choice>,
    pub mac: bool,
    pub cfg: Config,
    pub shadow: bool,
    pub name_valid: bool,
    pub taken: Option<String>,
    pub suggestions: Vec<&'static str>,
    pub commands: Vec<String>,
    pub installable: bool,
    pub omarchy_name: Option<String>,
    /// The palette of each look, in `LOOKS` order.
    pub looks: Vec<Palette>,
    toggles: Vec<(&'static str, bool)>,
}

/// Every key `Setup::toggle_on` knows.
const TOGGLES: &[&str] = &["leader", "import", "multi", "autoclose", "click", "drag", "double", "right", "path", "env", "git", "default"];

impl Model {
    fn of(s: &Setup, app: &App, system_dark: bool) -> Model {
        let choices = s.choices();
        Model {
            step: s.step,
            focus: choices.get(s.focus.min(choices.len().saturating_sub(1))).copied(),
            mac: s.mac,
            cfg: s.cfg.clone(),
            shadow: s.shadow,
            name_valid: s.name_valid(),
            taken: s.taken(),
            suggestions: s.suggestions(),
            commands: if s.step >= 4 { s.commands() } else { Vec::new() },
            installable: s.installable(),
            omarchy_name: s.omarchy_name.clone(),
            looks: LOOKS
                .iter()
                .map(|id| theme::resolve(theme::Source::parse(id), "dark", "light", system_dark, app.omarchy.as_ref()).0)
                .collect(),
            toggles: TOGGLES.iter().map(|k| (*k, s.toggle_on(k))).collect(),
        }
    }

    pub fn vim(&self) -> bool {
        self.cfg.keys.map != "classic"
    }

    pub fn mod_key(&self) -> &'static str {
        if self.mac { "⌘" } else { "Ctrl" }
    }

    pub fn on(&self, key: &str) -> bool {
        self.toggles.iter().any(|(k, v)| *k == key && *v)
    }

    pub fn focused(&self, c: Choice) -> bool {
        self.focus == Some(c)
    }
}

pub fn setup(s: Session, keys: ElementRef) -> impl IntoElement {
    let model = s.view(|a| match &a.screen {
        Screen::Setup(st) => Some(Model::of(st, a, dark())),
        Screen::Editor => None,
    });
    let vim = computed(move || model.with(|m| m.as_ref().is_none_or(Model::vim)));
    view()
        .flex_1()
        .bg(move || color("background"))
        .test_id("setup")
        .child(scroll().flex_1().child(view().w_full().items_center().pt(px(40.0)).pb(px(24.0)).px(px(32.0)).child(
            view().w_full().max_w(px(900.0)).gap(px(26.0)).child(header(model)).child(dynamic(move || match model.get() {
                Some(m) => setup_steps::body(s, keys, &m),
                None => view().into_element(),
            })),
        )))
        .child(dynamic(move || match model.get() {
            Some(m) => status(s, keys, &m),
            None => view().into_element(),
        }))
        .child(
            view().flex_row().h(px(22.0)).px(px(8.0)).items_center().child(
                text(move || if vim.get() { "j/k move   enter pick   tab next" } else { "arrows move   enter pick   tab next" })
                    .text_xs()
                    .text_color(move || color("muted-foreground")),
            ),
        )
}

fn header(model: Memo<Option<Model>>) -> impl IntoElement {
    let step = computed(move || model.with(|m| m.as_ref().map_or(1, |m| m.step)));
    view().flex_row().gap(px(14.0)).items_center().child(icon(34.0)).child(
        view().gap(px(2.0)).child(text("nsvim").text_base().font_weight(FontWeight::BOLD).text_color(move || color("foreground"))).child(
            text(move || format!("setup {}/{} · {}", step.get(), STEPS.len(), STEPS[step.get() - 1]))
                .text_xs()
                .text_color(move || color("muted-foreground"))
                .test_id("setup-step"),
        ),
    )
}

/// The app's icon: four lines in the mode colours.
pub fn icon(size: f32) -> impl IntoElement {
    let k = size / 128.0;
    let bar = move |x: f32, y: f32, w: f32, token: &'static str| {
        view().absolute().left(px(x * k)).top(px(y * k)).w(px(w * k)).h(px(12.0 * k)).rounded(px(6.0 * k)).bg(move || color(token))
    };
    view()
        .w(px(size))
        .h(px(size))
        .rounded(px(28.0 * k))
        .bg(move || color("muted"))
        .border_1()
        .border_color(move || color("border"))
        .label("nsvim")
        .child(bar(28.0, 32.0, 72.0, "primary"))
        .child(bar(28.0, 52.0, 50.0, "mode-insert"))
        .child(bar(40.0, 72.0, 60.0, "mode-visual"))
        .child(bar(28.0, 92.0, 34.0, "mode-command"))
}

/// The status line: SETUP, the steps, and skip, back and next.
fn status(s: Session, keys: ElementRef, m: &Model) -> AnyElement {
    let mut line = view().flex_row().h(px(24.0)).w_full().items_center().bg(move || color("muted")).child(
        view()
            .self_stretch()
            .justify_center()
            .px(px(12.0))
            .bg(move || color("primary"))
            .child(text("SETUP").text_xs().font_weight(FontWeight::BOLD).text_color(move || color("primary-foreground"))),
    );
    for (i, t) in STEPS.iter().enumerate() {
        let n = i + 1;
        let token = if n == m.step {
            "foreground"
        } else if n < m.step {
            "mode-insert"
        } else {
            "muted-foreground"
        };
        let mut label = text(format!("{}{t}", if i > 0 { "› " } else { "" })).text_xs().px(px(6.0)).text_color(move || color(token));
        if n == m.step {
            label = label.font_weight(FontWeight::BOLD);
        }
        line = line.child(label);
    }
    line = line.child(view().flex_1());
    let btn = |label: &'static str, go: bool, by: isize| {
        button()
            .accessibility_label(label)
            .on_press(act(s, keys, move |a, tx| if by == 99 { a.finish_setup(tx) } else { a.setup_go(by) }))
            .self_stretch()
            .flex_row()
            .px(px(12.0))
            .items_center()
            .bg(move || color(if go { "primary" } else { "muted" }))
            .hover(|st| st.opacity(0.85))
            .active(|st| st.opacity(0.7))
            .child(text(label).text_xs().text_color(move || color(if go { "primary-foreground" } else { "muted-foreground" })))
    };
    let last = STEPS.len();
    if m.step < last {
        line = line.child(btn("skip", false, 10));
    }
    if m.step > 1 {
        line = line.child(btn("back", false, -1));
    }
    if m.step < last {
        line = line.child(btn("next →", true, 1));
    } else {
        line = line.child(btn("open nsvim →", true, 99));
    }
    line.into_element()
}
