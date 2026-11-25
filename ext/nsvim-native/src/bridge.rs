//! The join between the Rust app and its TSX screens (`App.tsx`).
//!
//! Rust owns all app state. TSX holds only copies of parts of it, and asks Rust for changes:
//!
//! - **Listeners.** `subscribe` gives a TSX listener a part of the app (a `#[derive(JsObject)]`
//!   struct) now, and again after each update that changes it. TSX keeps the part in a signal.
//! - **Commands.** `send` queues a change for the session. The session's task runs it on the
//!   engine thread, through `Session::update`, then the listeners hear what changed.
//!
//! A TSX call into a module runs outside the reactive runtime, so it cannot read the session.
//! Listeners and commands therefore live here, in plain thread-local cells, and the session
//! takes them on its next poll. Each engine thread has one session.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use nsh_core::element::CallbackArgs;
use nsh_core::prelude::*;

use crate::app::App;
use crate::store::Session;

/// A listener of one part of the app.
trait Listener {
    /// Posts the part when it changed since the last post, or when `force` is set.
    fn deliver(&mut self, app: &App, force: bool);
}

struct Part<T, F> {
    part: F,
    last: Option<T>,
    listener: Callback<T>,
}

impl<T, F> Listener for Part<T, F>
where
    T: CallbackArgs + Clone + PartialEq + 'static,
    F: Fn(&App) -> T,
{
    fn deliver(&mut self, app: &App, force: bool) {
        let now = (self.part)(app);
        if force || self.last.as_ref() != Some(&now) {
            self.last = Some(now.clone());
            self.listener.post(now);
        }
    }
}

/// A change that TSX asks for. It runs on the engine thread with the session.
pub type Command = Box<dyn FnOnce(Session)>;

#[derive(Default)]
struct Bridge {
    next: u64,
    /// The listeners, by id. A listener without a last value has heard nothing yet.
    listeners: Vec<(u64, Rc<RefCell<dyn Listener>>)>,
    /// Listeners added since the last delivery. They hear their part on the next one.
    fresh: Vec<u64>,
    commands: VecDeque<Command>,
    /// Wakes the session's task. Set when the session starts.
    wake: Option<Rc<dyn Fn()>>,
}

thread_local! {
    static BRIDGE: RefCell<Bridge> = RefCell::new(Bridge::default());
}

fn wake() {
    if let Some(w) = BRIDGE.with(|b| b.borrow().wake.clone()) {
        w();
    }
}

/// Gives `listener` the part `part` of the app on the next poll, then after each update that
/// changes it. Returns the `Stop` that TSX calls (or its owner's end) to stop listening.
pub fn subscribe<T>(part: impl Fn(&App) -> T + 'static, listener: Callback<T>) -> Stop
where
    T: CallbackArgs + Clone + PartialEq + 'static,
{
    let id = BRIDGE.with(|b| {
        let mut b = b.borrow_mut();
        let id = b.next;
        b.next += 1;
        b.listeners.push((id, Rc::new(RefCell::new(Part { part, last: None, listener }))));
        b.fresh.push(id);
        id
    });
    wake();
    Stop::new(move || {
        BRIDGE.with(|b| {
            let mut b = b.borrow_mut();
            b.listeners.retain(|(i, _)| *i != id);
            b.fresh.retain(|i| *i != id);
        });
    })
}

/// Queues `command` for the session and wakes it.
pub fn send(command: impl FnOnce(Session) + 'static) {
    BRIDGE.with(|b| b.borrow_mut().commands.push_back(Box::new(command)));
    wake();
}

/// `send` for a change of the app through `Session::update`.
pub fn update(f: impl FnOnce(&mut App, &mut crate::app::fx::Tx) + 'static) {
    send(move |s| s.update(f));
}

/// Sets what wakes the session's task. `Session::start` calls it.
pub(crate) fn set_wake(w: impl Fn() + 'static) {
    BRIDGE.with(|b| b.borrow_mut().wake = Some(Rc::new(w)));
}

/// Runs the queued commands, in order. The session's task calls it before each poll.
pub(crate) fn run_commands(s: Session) {
    while let Some(c) = BRIDGE.with(|b| b.borrow_mut().commands.pop_front()) {
        c(s);
    }
}

/// Tells the listeners what changed in `app`. `Session::update` calls it after each update.
pub(crate) fn deliver(app: &App) {
    // The list is copied out first: a listener's TSX runs later (`post`), but a listener may
    // still be added while this runs.
    let (all, fresh) = BRIDGE.with(|b| {
        let mut b = b.borrow_mut();
        let fresh = std::mem::take(&mut b.fresh);
        (b.listeners.clone(), fresh)
    });
    for (id, l) in all {
        l.borrow_mut().deliver(app, fresh.contains(&id));
    }
}
