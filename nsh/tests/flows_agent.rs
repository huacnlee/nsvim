//! Agents drive the real app through the socket API: state, reads, edits, proposals, notes
//! and `open --wait`. The requests go to `App::ipc` as the socket worker sends them.

mod common;

use common::{SRC, rust_flow};
use serde_json::json;

#[test]
fn agents_read_state_and_unsaved_text_through_the_socket_api() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("jx");
    let st = f.ask("state", json!({}));
    assert!(st.ok);
    assert_eq!(st.result["active"]["line"], 2);
    assert_eq!(st.result["active"]["dirty"], true);
    let read = f.ask("read", json!({ "start_line": 2, "end_line": 2 }));
    assert_eq!(read.result["text"], "   let a = 1;\n", "the unsaved edit, not the disk copy");
    let bad = f.ask("nope", json!({}));
    assert!(!bad.ok && bad.error.unwrap().contains("unknown command"));
}

#[test]
fn an_agent_edit_is_one_undo_step_and_can_be_turned_off() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let new = SRC.replace("let b = 2;", "let b = 20;");
    let r = f.ask("edit", json!({ "text": new, "label": "bump b" }));
    assert!(r.ok, "{:?}", r.error);
    assert!(f.text().contains("let b = 20;"));
    f.keys("u");
    assert_eq!(f.text(), SRC);
    f.update(|a, _| a.cfg.ipc.allow_edits = false);
    let r = f.ask("edit", json!({ "text": "x" }));
    assert!(!r.ok);
    assert_eq!(f.text(), SRC);
}

#[test]
fn a_proposal_shows_until_y_accepts_or_n_rejects() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let new = SRC.replace("println!", "eprintln!");
    let r = f.ask("propose", json!({ "text": new, "label": "log to stderr" }));
    assert!(r.ok);
    assert_eq!(r.result["added"], 1);
    assert!(f.read(|a| a.proposal_here().is_some()));
    f.keys("j");
    assert_eq!(f.text(), SRC, "j scrolls the diff, it does not edit");
    f.keys("y");
    assert!(f.text().contains("eprintln!"));
    assert!(f.read(|a| a.proposals.is_empty()));
    f.keys("u");
    let r = f.ask("propose", json!({ "text": new }));
    assert!(r.ok);
    f.keys("n");
    assert_eq!(f.text(), SRC);
}

#[test]
#[ignore = "needs text"]
fn a_proposal_shows_its_diff_in_the_text_area() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let new = SRC.replace("println!", "eprintln!");
    assert!(f.ask("propose", json!({ "text": new, "label": "log to stderr" })).ok);
    f.ui.wait_for("text:/eprintln!/").unwrap();
    f.ui.wait_for("text:/log to stderr/").unwrap();
    f.keys("n");
    f.ui.wait_for_gone("text:/eprintln!/", std::time::Duration::from_secs(2)).unwrap();
}

#[test]
fn reveal_marks_lines_and_esc_clears_them() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let r = f.ask("reveal", json!({ "line": 3, "end_line": 4, "note": "these add up" }));
    assert!(r.ok);
    let n = f.read(|a| a.note.as_ref().map(|n| (n.first, n.last))).expect("a note");
    assert_eq!(n, (2, 3));
    assert!(f.said("these add up"), "{:?}", f.msg());
    f.ui.wait_for("text:/these add up/").unwrap();
    f.keys("\x1b");
    assert!(f.read(|a| a.note.is_none()));
}

#[test]
fn open_wait_answers_when_the_buffer_closes() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let path = f.read(|a| a.ed.buf().path.clone().unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let req = nsh_app_editor::ipc::Request { id: 9, cmd: "open".into(), args: json!({ "path": path, "wait": true }) };
    f.update(move |a, t| a.ipc(req, tx, t));
    f.on_engine(|s| s.poll());
    assert!(rx.try_recv().is_err(), "held while the buffer is open");
    f.keys(":q\n");
    f.on_engine(|s| s.poll());
    let r = rx.recv_timeout(std::time::Duration::from_secs(5)).expect("answered after :q");
    assert_eq!(r.result["closed"], true);
}
