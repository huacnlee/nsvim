//! `Workspace`: the editor window's content, around the TSX screens.

use nsh_core::KeyCapture;
use nsh_core::desktop::{QuitMode, WindowFrame};
use nsh_core::element::desktop::{current_window, on_app_quit, on_quit_request, set_quit_mode};
use nsh_core::geometry::Points;
use nsh_core::prelude::*;

use crate::store::Session;
use crate::{app, keys, opacity, ui, window_attributes};

/// The props of [`Workspace`].
#[derive(Clone, PartialEq, Props)]
pub struct WorkspaceProps {
    /// The screens.
    pub children: Prop<Children>,
}

/// The editor window's content. It opens the editor as the command line asks, follows the
/// theme, runs the workers, and takes every key. Put the screens inside it.
#[component]
pub fn Workspace(props: WorkspaceProps) -> impl IntoElement {
    let pending = crate::take_launch();
    let real = pending.is_none();
    let s = Session::new(make_app(pending.as_ref()));
    if let Some(f) = pending.and_then(|p| p.on_session) {
        f(s);
    }
    let keys = element_ref();
    super::set_current(s, keys);

    // The colours follow config.toml's theme, the setup's pick and the system appearance.
    let light = s.view(|a| ui::palette(a, false));
    let dark_palette = s.view(|a| ui::palette(a, true));
    let alpha = s.view(|a| opacity(&a.cfg));
    effect(move || {
        let mut p = if dark() { dark_palette.get() } else { light.get() };
        // Below full opacity, the background lets the desktop and its blur show through.
        p.bg = p.bg.alpha(p.bg.a * alpha.get());
        set_tokens(ui::style::tokens(&p));
    });

    let win = current_window();
    let attributes = s.view(|a| window_attributes(&a.cfg));
    {
        let win = win.clone();
        effect(move || win.set(attributes.get()));
    }
    // A real launch opens at the size the window had when it last closed.
    if real && let Some(f) = app::session::load_window() {
        win.set_bounds(WindowFrame { width: Some(Points(f.w)), height: Some(Points(f.h)), ..Default::default() });
    }
    // The editor quits when its window closes, on macOS too.
    set_quit_mode(QuitMode::LastWindowClosed);
    // The platform's quit key and Quit item ask first, as the close button does.
    on_quit_request(move || s.close_requested());
    // However the app ends (the system logs out), the session is written once.
    on_app_quit(move || {
        s.update(|a, _| a.exit());
        async {}
    });
    s.attach(win);
    s.start();
    if !crate::terminal() {
        // The menu follows the keymap and Settings: they decide the shortcut of Close Window.
        let menu_keys = s.view(|a| a.close_window_key());
        effect(move || {
            menu_keys.get();
            untracked(|| set_app_menu(s));
        });
    }

    // A view takes the focus only once its window shows, which comes after the first layout on
    // Linux. Ask until it holds the focus, for at most 5 seconds. On the web the editor is often
    // a frame in a larger page, and it must not take the focus from that page. There it takes the
    // focus when the user clicks it.
    #[cfg(not(target_arch = "wasm32"))]
    {
        spawn(async move {
            for _ in 0..50 {
                if s.read(|a| a.key_focus) {
                    break;
                }
                keys.focus();
                timer(std::time::Duration::from_millis(100)).await;
            }
        });
    }

    // The root paints the background once: a translucent one painted twice would be darker.
    let root_bg = move || {
        if alpha.get() < 1.0 { ui::style::swatch(crate::color::Color::TRANSPARENT) } else { ui::style::color("background") }
    };
    view()
        .flex_1()
        .bg(root_bg)
        .text_color(move || ui::style::color("foreground"))
        .element_ref(&keys)
        .focusable(true)
        .capture_keys(KeyCapture::All)
        .on_focus(move || s.update(|a, _| a.key_focus = true))
        .on_blur(move || s.update(|a, _| a.key_focus = false))
        .on_key_down(move |e: &KeyEvent| {
            if let Some(k) = keys::convert(e, apple_keys()) {
                s.key(k);
            }
        })
        .child(props.children)
}

/// The app state for a launch that a test set, or else for the command line (or, in a
/// browser, the demo project).
fn make_app(p: Option<&crate::Pending>) -> app::App {
    let Some(p) = p else {
        #[cfg(target_arch = "wasm32")]
        return crate::services::web::demo_app();
        #[cfg(not(target_arch = "wasm32"))]
        return app::App::init();
    };
    let mut a = crate::app_for(&p.launch);
    if p.launch.project.is_some() {
        a.config_file = p.config_file.clone();
    }
    a
}

/// The application menu (`app::app_menu`), each item carried out on the session. The recent
/// projects are read as the menu is made: as the window opens, and when the keymap or Settings
/// change Close Window's shortcut.
#[cfg(not(target_arch = "wasm32"))]
fn set_app_menu(s: Session) {
    use nsh_core::desktop::MenuRole;
    use nsh_core::element::desktop::{MenuItem, set_application_menu};

    fn build(s: Session, entries: Vec<app::app_menu::Entry>) -> Vec<MenuItem> {
        use app::app_menu::Entry;
        entries
            .into_iter()
            .map(|e| match e {
                Entry::Menu(label, items) => MenuItem::submenu(label, build(s, items)),
                Entry::Item(id, label) if id == "none" => MenuItem::new(label).disabled(true),
                Entry::Item(id, label) => MenuItem::new(label).on_select(move || s.update(|a, tx| a.app_menu_pick(&id, tx))),
                Entry::Quit => MenuItem::role(MenuRole::Quit),
                Entry::Hide(label, key) => {
                    let item = MenuItem::role(MenuRole::Hide).label(label);
                    match key {
                        Some(key) => item.shortcut(&key),
                        None => item,
                    }
                }
                Entry::HideOthers => MenuItem::role(MenuRole::HideOthers),
                Entry::ShowAll => MenuItem::role(MenuRole::ShowAll),
                Entry::Separator => MenuItem::separator(),
            })
            .collect()
    }
    let entries = s.read(|a| a.app_menu(&app::session::recent(&a.ed.root), cfg!(target_os = "macos")));
    set_application_menu(build(s, entries));
}

#[cfg(target_arch = "wasm32")]
fn set_app_menu(_: Session) {}
