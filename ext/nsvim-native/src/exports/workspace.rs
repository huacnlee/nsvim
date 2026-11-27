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
    on_quit_request(move || {
        s.close_requested();
        super::refocus_keys();
    });
    // However the app ends (the system logs out), the session is written once.
    on_app_quit(move || {
        s.update(|a, _| a.exit());
        async {}
    });
    #[cfg(not(target_arch = "wasm32"))]
    let focus_win = win.clone();
    s.attach(win);
    s.start();
    crate::ui::popups::host_events(s);
    if !crate::terminal() {
        // The menu follows the keymap and Settings, which decide the shortcut of Close Window,
        // and the state of View's check marks, however it changes: a key, the palette or the menu.
        let menu_keys = s.view(|a| a.app_menu_state());
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

    // Every key goes to this view or to the hidden input below, and no other view takes the
    // keyboard. When the window becomes the key window again with neither focused (a close
    // press or a native dialog can leave it so), the keys come back here: else `:` and every
    // other key do nothing until the user clicks the text.
    #[cfg(not(target_arch = "wasm32"))]
    if !crate::terminal() {
        let win = focus_win;
        let was = signal(true);
        effect(move || {
            let focused = win.state().is_some_and(|w| w.focused);
            if focused && !was.get_untracked() && !s.read(|a| a.key_focus) {
                keys.focus();
            }
            was.set(focused);
        });
    }

    // The root paints the background once: a translucent one painted twice would be darker.
    let root_bg = move || {
        if alpha.get() < 1.0 { ui::style::swatch(crate::color::Color::TRANSPARENT) } else { ui::style::color("background") }
    };
    // While the hidden input has the keyboard, the root captures nothing: an ancestor's capture
    // would take the characters before the input method sees them.
    let sink_focused = signal(false);
    // Where the root is in the window: the hidden input is placed from the text area's corner.
    let root_at = signal((0.0_f32, 0.0_f32));
    view()
        .flex_1()
        .bg(root_bg)
        .text_color(move || ui::style::color("foreground"))
        .element_ref(&keys)
        .on_layout(move |_: &Bounds<Points>| {
            spawn(async move {
                if let Some(m) = keys.measure().await {
                    root_at.set((m.page.x.0, m.page.y.0));
                }
            });
        })
        .focusable(true)
        .capture_keys(move || if sink_focused.get() { KeyCapture::None } else { KeyCapture::All })
        .on_focus(move || s.update(|a, _| a.key_focus = true))
        .on_blur(move || s.update(|a, _| a.key_focus = false))
        .on_key_down(move |e: &KeyEvent| {
            if sink_focused.get_untracked() {
                return;
            }
            if let Some(captured) = keys::mac_quit_key(e, os()) {
                if captured {
                    s.close_requested();
                }
                return;
            }
            if let Some(k) = keys::convert(e, apple_keys()) {
                s.key(k);
            }
        })
        .child(props.children)
        .child(text_sink(s, keys, sink_focused, root_at))
}

/// The hidden input that takes the keyboard where text is typed (Insert mode, the classic
/// keymap), so the platform's input method can compose there ([`crate::ime`]). Keys that do not
/// type text are captured and go to the editor as before; nothing is captured while an input
/// method composes, whose keys (Backspace, Enter, the arrows) are its own. It sits at the
/// cursor, one row tall, so the input method's candidates show by the cursor. Not in a terminal.
fn text_sink(s: Session, keys: ElementRef, focused: Signal<bool>, root_at: Signal<(f32, f32)>) -> AnyElement {
    use editor::editor::{KeyCode, KeyInput, Keymap, Mode};
    use std::{cell::RefCell, rc::Rc};

    let sink = element_ref();
    if crate::terminal() {
        return view().into_element();
    }
    let typing = s.view(|a| a.ed.keymap != Keymap::Vim || a.ed.mode == Mode::Insert);
    let editor = s.view(|a| a.key_focus);
    let composing = signal(false);
    let text = Rc::new(RefCell::new(crate::ime::SinkText::default()));
    // Keys that do not type text go to the editor through the keymap while the input has the
    // keyboard: a focused input owns its raw keys. Not while an input method composes.
    let bound = Rc::new(crate::ime::editor_keys(apple_keys()));
    effect(move || {
        if !focused.get() || composing.get() {
            return;
        }
        let os = os();
        for (text, k) in bound.iter() {
            let reserved = nsh_core::keymap::Keystroke::parse_with(text, apple_keys()).map_or(true, |p| p.reserved(os).is_some());
            if reserved {
                continue;
            }
            let k = *k;
            on_key_with(
                text,
                KeyOptions { when: nsh_core::keymap::When::Always, repeat: true, for_focus: true, ..Default::default() },
                move |_| s.key(k),
            );
        }
    });
    // The keyboard follows the mode: to the input while text is typed, back to the editor after.
    {
        effect(move || {
            let (typing, editor) = (typing.get(), editor.get());
            if typing && editor && !focused.get() {
                sink.focus();
            } else if !typing && focused.get() {
                keys.focus();
            }
        });
    }
    let typed = move |t: String| {
        for c in t.chars() {
            let code = match c {
                '\n' | '\r' => KeyCode::Enter,
                '\t' => KeyCode::Tab,
                c => KeyCode::Char(c),
            };
            s.key(KeyInput { code, ctrl: false, alt: false, shift: false, cmd: false });
        }
    };
    // The cursor's cell in the root: the text area's corner, then the cell's row and column.
    let spot = computed(move || {
        let (ax, ay) = s.area_at.get()?;
        let (rx, ry) = root_at.get();
        let (caret, gutter) = s.frame.with(|f| (f.caret, f.gutter));
        let (c, g) = (caret?, s.grid());
        Some(((gutter + c.col) as f32 * g.cell_w + ax - rx, c.row as f32 * g.cell_h + ay - ry, g.cell_h))
    });
    let (change, compose) = (text.clone(), text);
    nsh_core::element::input()
        .element_ref(&sink)
        .test_id("text-input")
        .absolute()
        .left(move || px(spot.get().map_or(0.0, |p| p.0)))
        .top(move || px(spot.get().map_or(0.0, |p| p.1)))
        .w(px(1.0))
        .h(move || px(spot.get().map_or(1.0, |p| p.2)))
        .pointer_events(PointerEvents::None)
        .p_0()
        .border(px(0.0))
        .bg(AdaptiveColor::TRANSPARENT)
        .text_color(AdaptiveColor::TRANSPARENT)
        // Not 0, which takes the input out of accessibility and focus.
        .opacity(0.02)
        .caret(false)
        .auto_correct(false)
        .value(String::new())
        .on_change(move |t: &str| typed(change.borrow_mut().changed(t)))
        .on_composition_change(move |c: &nsh_core::element::Composition| {
            composing.set(c.composing);
            typed(compose.borrow_mut().composed(c.composing));
        })
        .on_focus(move || {
            focused.set(true);
            s.update(|a, _| a.key_focus = true);
        })
        .on_blur(move || {
            focused.set(false);
            s.update(|a, _| a.key_focus = false);
        })
        .into_element()
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
/// projects are read as the menu is made: as the window opens, and when Close Window's shortcut
/// or a check mark of View changes.
#[cfg(not(target_arch = "wasm32"))]
fn set_app_menu(s: Session) {
    let entries = s.read(|a| a.app_menu(&app::session::recent(&a.ed.root), cfg!(target_os = "macos")));
    nsh_core::element::desktop::set_application_menu(crate::ui::native_menu::app_menu_items(s, entries, false));
}

#[cfg(target_arch = "wasm32")]
fn set_app_menu(_: Session) {}
