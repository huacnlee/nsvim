use super::*;
use crate::vim::tests::{ed, text};

const PLAIN: FindOpts = FindOpts { case: false, word: false, regex: false };

#[test]
fn plain_text_matches_literally_and_ignores_case() {
    let e = ed("a.b axb A.B\n");
    let p = pattern("a.b", PLAIN);
    assert_eq!(count(&e, &p), (Some(1), 2), "the cursor is on the first match");
    let p = pattern("a.b", FindOpts { case: true, ..PLAIN });
    assert_eq!(count(&e, &p).1, 1);
    // Chars the pattern syntax reads stay plain.
    let e = ed("[x] $y ^z a/b \\w\n");
    for q in ["[x]", "$y", "^z", "a/b", "\\w"] {
        assert_eq!(count(&e, &pattern(q, PLAIN)).1, 1, "{q}");
    }
}

#[test]
fn whole_word_and_regex() {
    let e = ed("cat catalog cat_x cat\n");
    assert_eq!(count(&e, &pattern("cat", FindOpts { word: true, ..PLAIN })).1, 2);
    let re = FindOpts { regex: true, ..PLAIN };
    let e = ed("id1 id22 idx (a|b)\n");
    assert_eq!(count(&e, &pattern("id\\d+", re)).1, 2);
    assert_eq!(count(&e, &pattern("id\\d{2}", re)).1, 1);
    assert_eq!(count(&e, &pattern("(id|x)\\d?", re)).1, 4);
    assert_eq!(count(&e, &pattern("\\(a\\|b\\)", re)).1, 1);
    assert_eq!(count(&e, &pattern("(?:id)x", re)).1, 1);
    // A pattern that does not compile matches nothing.
    assert_eq!(count(&e, &pattern("(id", re)).1, 0);
}

#[test]
fn step_wraps_both_ways_and_counts_the_current_match() {
    let mut e = ed("one two\none\ntwo one\n");
    let p = pattern("one", PLAIN);
    assert!(first_from(&mut e, &p, Pos::new(0, 0)));
    assert_eq!(count(&e, &p), (Some(1), 3));
    assert!(step(&mut e, &p, false));
    assert_eq!(e.cursor(), Pos::new(1, 0));
    assert!(step(&mut e, &p, false));
    assert_eq!((e.cursor(), count(&e, &p).0), (Pos::new(2, 4), Some(3)));
    assert!(step(&mut e, &p, false));
    assert_eq!(e.cursor(), Pos::new(0, 0), "wraps to the top");
    assert!(step(&mut e, &p, true));
    assert_eq!(e.cursor(), Pos::new(2, 4), "wraps to the bottom");
    assert!(!step(&mut e, &pattern("zzz", PLAIN), false));
}

#[test]
fn the_classic_keymap_selects_the_match() {
    let mut e = ed("say hello\n");
    e.set_keymap(Keymap::Classic);
    assert!(first_from(&mut e, &pattern("hello", PLAIN), Pos::new(0, 0)));
    assert_eq!((e.doc().view.anchor, e.cursor()), (Some(Pos::new(0, 4)), Pos::new(0, 9)));
    assert_eq!(count(&e, &pattern("hello", PLAIN)), (Some(1), 1));
}

#[test]
fn replace_one_then_all_each_undo_as_one_step() {
    let mut e = ed("a1 a2 a3\n");
    let p = pattern("a", PLAIN);
    assert!(first_from(&mut e, &p, Pos::new(0, 0)));
    assert!(replace_one(&mut e, &p, &replacement("b&\\", false)));
    assert_eq!(text(&e), "b&\\1 a2 a3\n");
    assert_eq!(e.cursor(), Pos::new(0, 5), "goes to the next match");
    assert_eq!(replace_all(&mut e, &p, &replacement("c", false)), 2);
    assert_eq!(text(&e), "b&\\1 c2 c3\n");
    e.doc_mut().buf.undo();
    assert_eq!(text(&e), "b&\\1 a2 a3\n", "replace all is one undo step");
    e.doc_mut().buf.undo();
    assert_eq!(text(&e), "a1 a2 a3\n");
}

#[test]
fn regex_replacement_takes_groups() {
    let mut e = ed("key=value\nx=y\n");
    let re = FindOpts { regex: true, ..PLAIN };
    let p = pattern("(\\w+)=(\\w+)", re);
    assert_eq!(replace_all(&mut e, &p, &replacement("$2: $1 ($&) $$", true)), 2);
    assert_eq!(text(&e), "value: key (key=value) $\ny: x (x=y) $\n");
}
