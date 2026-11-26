//! Tests of the documents' order: a tab dragged along the buffer line.

use std::path::PathBuf;

use super::{Doc, Editor};
use crate::buffer::Buffer;

/// An editor with one document per name, each holding its name, and document `active` active.
fn docs(names: &[&str], active: usize) -> Editor {
    let mut e = Editor::new(PathBuf::from("/project"));
    for n in names {
        e.docs.push(Doc::new(Buffer::from_str(n)));
    }
    e.active = active;
    e
}

/// The documents' texts in order, and the active one's text.
fn order(e: &Editor) -> (Vec<String>, String) {
    let all = e.docs.iter().map(|d| d.buf.slice(0, d.buf.len_chars())).collect();
    (all, e.docs[e.active].buf.slice(0, e.docs[e.active].buf.len_chars()))
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_moved_document_takes_its_new_place_and_the_active_one_stays_active() {
    // The active document moves.
    let mut e = docs(&["a", "b", "c", "d"], 1);
    e.move_doc(1, 3);
    assert_eq!(order(&e), (strs(&["a", "c", "d", "b"]), "b".into()));
    // A document moves from before the active one to after it.
    let mut e = docs(&["a", "b", "c", "d"], 2);
    e.move_doc(0, 3);
    assert_eq!(order(&e), (strs(&["b", "c", "d", "a"]), "c".into()));
    // A document moves from after the active one to before it.
    let mut e = docs(&["a", "b", "c", "d"], 1);
    e.move_doc(3, 0);
    assert_eq!(order(&e), (strs(&["d", "a", "b", "c"]), "b".into()));
    // A move on one side of the active document leaves it where it is.
    let mut e = docs(&["a", "b", "c", "d"], 0);
    e.move_doc(3, 1);
    assert_eq!(order(&e), (strs(&["a", "d", "b", "c"]), "a".into()));
}

#[test]
fn a_move_to_the_same_place_or_past_the_end_does_nothing() {
    for (from, to) in [(1, 1), (4, 0), (0, 4)] {
        let mut e = docs(&["a", "b", "c", "d"], 2);
        e.move_doc(from, to);
        assert_eq!(order(&e), (strs(&["a", "b", "c", "d"]), "c".into()), "{from} to {to}");
    }
}
