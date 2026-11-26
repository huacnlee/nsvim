//! The app's one store. `App` (all editor state, in `crate::app`) lives in a `stored` cell,
//! out of the reactive graph. Every change goes through `Session::update`, which bumps a
//! revision signal and then carries out the effects the app recorded on its `Tx`.
//!
//! Views never read `App` in a body. Each view makes a memo of the part it shows with
//! `Session::view`, so a key changes only the views whose part changed.
//!
//! The workers (`crate::services`) send events on a channel and call a waker. The waker sends
//! on a tokio channel that a task on the engine thread awaits, and that task polls the app.

use std::path::PathBuf;
use std::time::Duration;

// `std::time::Instant` has no clock on the web. This one is the same type everywhere else.
use nsh_core::time::Instant;

use editor::editor::KeyInput;
use nsh_core::desktop::WindowAttributes;
use nsh_core::element::desktop::{WindowRef, current_window, quit};
use nsh_core::prelude::*;

use crate::app::App;
use crate::app::fx::{Clip, Drag, Effect, Mods, Phase, PointerButton, Tx, WheelEvent};
use crate::model::{self, Frame};
use crate::ui::style::CodeFont;

/// The text area's size in cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// The advance of one char of the code font.
    pub cell_w: f32,
    /// The height of one row.
    pub cell_h: f32,
    pub rows: usize,
    pub cols: usize,
}

/// The store. `Copy`: capture it in closures freely.
#[derive(Clone, Copy)]
pub struct Session {
    app: Stored<App>,
    rev: Signal<u64>,
    grid: Signal<Grid>,
    started: Stored<Instant>,
    /// What the press of a drag selects by (`Drag::data`).
    drag_data: Stored<f32>,
    /// The one wake timer that is set, and when it fires (`wake_after`).
    wake: Stored<Option<(Instant, Spawned<()>)>>,
    /// The owner of the session. Wake timers run under it, so they live as long as the
    /// session and not only as long as the view that made the update.
    owner: Owner,
    /// The editor's window, once its content is made (`attach`).
    win: Stored<Option<WindowRef>>,
    /// What the text area shows.
    pub frame: Memo<Frame>,
    /// The code font, from config.toml.
    pub font: Memo<CodeFont>,
}

impl Session {
    pub fn new(app: App) -> Session {
        let app = stored(app);
        let rev = signal(0_u64);
        let font = computed(move || {
            rev.get();
            app.with(|a| CodeFont {
                family: a.font_family,
                size: a.cfg.font.size,
                row_h: crate::geometry::row_height(a),
                weight: a.cfg.font.weight,
            })
        });
        let grid = signal(Grid { cell_w: 7.8, cell_h: 22.0, rows: 30, cols: 100 });
        let frame = computed(move || {
            rev.get();
            let rows = grid.with(|g| g.rows);
            app.with(|a| model::app_frame(a, rows))
        });
        Session {
            app,
            rev,
            grid,
            started: stored(Instant::now()),
            drag_data: stored(0.0),
            wake: stored(None),
            owner: Owner::current(),
            win: stored(None),
            frame,
            font,
        }
    }

    /// A memo of the part of the app a view shows. It runs again after each update, and its
    /// readers run again only when the part changed.
    pub fn view<T: Clone + PartialEq + 'static>(&self, part: impl Fn(&App) -> T + 'static) -> Memo<T> {
        let (app, rev) = (self.app, self.rev);
        computed(move || {
            rev.get();
            app.with(|a| part(a))
        })
    }

    pub fn grid(&self) -> Grid {
        self.grid.get()
    }

    /// Seconds since the session started: the app's clock.
    pub fn clock(&self) -> f64 {
        self.started.with(|s| s.elapsed().as_secs_f64())
    }

    /// Reads the app without subscribing. For listeners and tests.
    pub fn read<R>(&self, f: impl FnOnce(&App) -> R) -> R {
        self.app.with(|a| f(a))
    }

    /// Changes the app, tells the views, then carries out what the app asked of the window.
    pub fn update<R>(&self, f: impl FnOnce(&mut App, &mut Tx) -> R) -> R {
        self.update_with(Tx::new(), f)
    }

    /// `update` for an input event with these modifiers and this button.
    pub fn update_with<R>(&self, mut tx: Tx, f: impl FnOnce(&mut App, &mut Tx) -> R) -> R {
        // The app's clock is the time of this update, so an edit records the time it happened.
        let clock = self.clock();
        let edited_at = self.app.with(|a| {
            a.clock = clock;
            a.last_edit
        });
        let out = self.app.with(|a| f(a, &mut tx));
        // Only a poll runs the work that follows an edit: the title's mark, the language
        // server's sync, the git signs and the swap file. Each asks for its own later wake.
        // Nothing else polls after a key, so an edit asks for a poll now.
        if self.app.with(|a| a.last_edit != edited_at) {
            tx.wake_after(Duration::ZERO);
        }
        self.rev.update(|n| *n += 1);
        // The TSX screens hear the parts of the app that changed.
        self.app.with(|a| crate::bridge::deliver(a));
        // NSH takes no activation token from the app. Without one, a Wayland compositor marks
        // the window as wanting attention.
        let raise = self.app.with(|a| a.raise.take());
        if raise.is_some() {
            let win = self.window();
            win.focus();
            win.request_attention(false);
        }
        for e in tx.effects {
            self.run(e);
        }
        out
    }

    /// Keeps the editor's window, for the title, raise and quit. Call it in the window's
    /// content, where `current_window()` is that window. Its size and place go into the
    /// session as they change.
    pub fn attach(&self, win: WindowRef) {
        let me = *self;
        win.on_bounds_change(move |_| me.window_moved());
        self.win.with(|w| *w = Some(win));
        self.window_moved();
    }

    /// The editor's window, or the app's when the content is not made yet.
    fn window(&self) -> WindowRef {
        self.win.with(|w| w.clone()).unwrap_or_else(current_window)
    }

    /// Keeps the window's size and place for `window.toml`.
    fn window_moved(&self) {
        let Some(st) = untracked(|| self.window().state()) else { return };
        let frame = crate::app::session::WindowFrame {
            w: st.content_bounds.size.width.0,
            h: st.content_bounds.size.height.0,
            x: Some(st.bounds.origin.x.0.round() as i32),
            y: Some(st.bounds.origin.y.0.round() as i32),
        };
        self.app.with(|a| a.window = frame);
    }

    /// The user pressed the window's close button, or asked to quit. The app decides: it
    /// quits, or warns about unsaved edits first.
    pub fn close_requested(&self) {
        let clock = self.clock();
        self.update(|a, tx| {
            a.clock = clock;
            a.close_requested(tx)
        });
    }

    fn run(&self, effect: Effect) {
        let me = *self;
        match effect {
            Effect::Quit => {
                // The app wrote the session already (`App::exit`).
                me.window().close();
                quit();
            }
            Effect::SetTitle(title) => {
                let mut attrs = WindowAttributes::default();
                attrs.title = Some(title);
                me.window().set(attrs);
            }
            Effect::Copy(text) => drop(set_clipboard(text)),
            Effect::ReadClipboard(then) => {
                spawn(async move {
                    let text = clipboard_string().await;
                    me.update(|a, tx| then(a, Clip(text), tx));
                });
            }
            Effect::PopupMenu { menu, x, y, then } => crate::ui::native_menu::native_menu(me, menu, x, y, then),
            Effect::WakeAfter(delay) => me.wake_after(delay),
            Effect::OpenProject(root) => {
                // The same as `nvim <folder>`: an editor already running for the project
                // takes it, else a new one opens.
                let started = std::env::current_exe().and_then(|exe| std::process::Command::new(exe).arg(&root).spawn());
                if let Err(e) = started {
                    me.update(|a, _| a.ed.error(format!("could not open {}: {e}", root.display())));
                }
            }
            Effect::ChooseFolder(start, then) => {
                let dialog = nsh_core::file_dialog::OpenDialog::folders().title("Open project").button_label("Open").directory(start);
                let answer = nsh_core::element::pick_paths(dialog);
                spawn(async move {
                    let picked = answer.await.map(|p| p.and_then(|v| v.into_iter().next()));
                    me.update(|a, tx| then(a, picked, tx));
                });
            }
            Effect::ChooseSavePath(start, then) => {
                let dialog = nsh_core::file_dialog::SaveDialog::new().directory(start);
                let answer = nsh_core::element::pick_save_path(dialog);
                spawn(async move {
                    let picked = answer.await;
                    me.update(|a, tx| then(a, picked, tx));
                });
            }
            Effect::Confirm(dialog, then) => {
                use crate::app::fx::Role;
                let mut alert = nsh_base::alert_dialog().title(dialog.title).description(dialog.description);
                for (id, label, role) in dialog.buttons {
                    alert = match role {
                        Role::Default => alert.action(id, label),
                        Role::Cancel => alert.cancel(id, label),
                        Role::Destructive => alert.destructive(id, label),
                    };
                }
                let answer = alert.show();
                spawn(async move {
                    let chosen = answer.await.map(|r| r.action.map(|a| a.to_string()));
                    me.update(|a, tx| then(a, chosen, tx));
                });
            }
        }
    }

    /// Polls the app after `delay`. One timer at a time: each poll asks again for what it
    /// still waits for, so one timer is enough. A timer that fires as soon or sooner keeps its
    /// place. A later one is cancelled for the new one. Before, each ask set a timer of its
    /// own, and every poll asked for up to three more, so the polls piled up.
    fn wake_after(&self, delay: Duration) {
        let at = nsh_core::time::now() + delay;
        if self.wake.with(|w| w.as_ref().is_some_and(|(t, _)| *t <= at)) {
            return;
        }
        let me = *self;
        let timer = self.owner.run(|| {
            set_timeout(delay, move || {
                me.wake.with(|w| *w = None);
                me.poll();
            })
        });
        if let Some((_, old)) = self.wake.with(|w| w.replace((at, timer))) {
            old.cancel();
        }
    }

    /// Applies what the workers sent.
    pub fn poll(&self) {
        let clock = self.clock();
        self.update(|a, tx| a.poll(clock, tx));
    }

    /// Starts the workers. Their events wake a task here that polls the app.
    pub fn start(&self) {
        let (wake_tx, mut wake_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let ui_wake = wake_tx.clone();
        crate::bridge::set_wake(move || {
            let _ = ui_wake.send(());
        });
        let waker: crate::services::Waker = std::sync::Arc::new(move || {
            let _ = wake_tx.send(());
        });
        self.update(|a, _| a.setup(waker));
        let me = *self;
        spawn(async move {
            while wake_rx.recv().await.is_some() {
                // Many events can come at once. One poll takes them all.
                while wake_rx.try_recv().is_ok() {}
                // What the TSX screens asked for, then what the workers sent.
                crate::bridge::run_commands(me);
                me.poll();
            }
        });
        // The first poll sets the title and opens what is ready.
        set_timeout(Duration::from_millis(1), move || me.poll());
    }

    /// A key, through the app's routing: setup, overlays, command line, tree, keymap.
    pub fn key(&self, key: KeyInput) {
        self.update(|a, tx| a.key(key, tx));
    }

    /// The text area got a new size, or the font changed: fit the grid to it.
    pub fn resize(&self, width: f32, height: f32, cell_w: f32) {
        let cell_h = self.font.with_untracked(|f| f.row_h);
        let gutter = self.read(crate::geometry::gutter_cols) as f32 * cell_w;
        let rows = ((height / cell_h).floor() as usize).max(1);
        let cols = (((width - gutter) / cell_w).floor() as usize).max(10);
        let g = Grid { cell_w, cell_h, rows, cols };
        if g == self.grid.get_untracked() {
            return;
        }
        batch(|| {
            self.grid.set(g);
            self.update(|a, _| {
                a.m.cw = cell_w;
                a.m.row_h = cell_h;
                a.m.gutter = gutter;
                a.m.text[2] = width;
                a.m.text[3] = height;
                a.ed.rows = rows;
                a.ed.cols = cols;
                if a.ed.has_doc() {
                    a.ed.scroll_to_cursor();
                }
            });
        });
    }

    /// A press, drag or release on the text area, in points from its top left.
    pub fn drag(&self, x: f32, y: f32, phase: Phase, clicks: u32, mods: Mods) {
        let mut d = Drag { x, y, start_x: x, start_y: y, phase, clicks, data: self.drag_data.with(|v| *v) };
        self.update_with(Tx::with_input(mods, PointerButton::Primary), |a, tx| a.text_drag(&mut d, tx));
        self.drag_data.with(|v| *v = d.data);
    }

    /// A wheel turn over the text area. `dy` is in points, positive scrolls down.
    pub fn wheel(&self, dx: f32, dy: f32, mods: Mods) {
        let e = WheelEvent { dx: -dx, dy: -dy, mods };
        self.update(|a, tx| a.text_wheel(e, tx));
    }

    /// Opens `path`, as `:e path` does.
    pub fn open(&self, path: PathBuf) {
        self.update(|a, _| a.open_path(path, None));
    }
}
