//! Timings for performance work. Each bench prints its timings and fails when one is over its
//! budget. Run them in release:
//! `TMPDIR=~/.cache/editor-test-tmp cargo test --release --test flows_bench -- --ignored --nocapture`.
//! The budgets are loose, so a slow machine passes. They catch a change that makes a step many
//! times slower.

mod common;

use std::time::Instant;

use common::{Flow, flow, keymap_config, scratch, write};
use editor::buffer::{Buffer, Pos};
use editor::editor::Doc;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// The fastest of `n` runs of `f`, in milliseconds, and the last result. The fastest run is
/// the one other processes disturbed least.
fn best_of<T>(n: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut best = f64::MAX;
    let mut last = None;
    for _ in 0..n {
        let t = Instant::now();
        last = Some(f());
        best = best.min(ms(t));
    }
    (best, last.expect("n > 0"))
}

/// Fails when `took` ms is over `budget` ms.
fn budget(what: &str, took: f64, budget: f64) {
    assert!(took <= budget, "{what}: {took:.1} ms is over its budget of {budget:.0} ms");
}

/// The app's own source, repeated to 200,000 lines.
fn big_source() -> String {
    let one = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap();
    one.repeat(200_000 / one.lines().count() + 1)
}

/// The time each char of `keys` takes, from the press until the app is quiet: the worst one.
fn keys(f: &mut Flow, keys: &str) -> f64 {
    let mut worst: f64 = 0.0;
    for c in keys.chars() {
        let t = Instant::now();
        f.keys(&c.to_string());
        worst = worst.max(ms(t));
    }
    worst
}

fn stroke(f: &mut Flow, stroke: &str) -> f64 {
    let t = Instant::now();
    f.press(stroke);
    ms(t)
}

/// From the engine's start to the file's text on screen, with the configured font and with a
/// named font only.
#[test]
#[ignore]
fn bench_first_frame() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap();
    for (name, family) in
        [("configured font", "JetBrains Mono"), ("named font", "IBM Plex Mono"), ("configured font, again", "JetBrains Mono")]
    {
        let dir = scratch();
        let file = write(dir.path(), "mod.rs", &src);
        let config = format!("{}[font]\nfamily = \"{family}\"\n", common::CONFIG);
        let t = Instant::now();
        let mut f = common::flow_files(&[&file], Some(&config));
        let started = ms(t);
        f.wait_open(&file);
        f.ui.wait_for("text:/^\\/\\/! The app's state/").unwrap();
        let shown = ms(t);
        eprintln!("{name}: engine and window {started:.1} ms, file on screen {shown:.1} ms");
        budget("the file on screen", shown, 3000.0);
    }
}

/// The text area's frame of 40 rows at line 1000 of a long file: highlight and layout of what
/// shows.
#[test]
#[ignore]
fn bench_highlight_window() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap().repeat(4);
    let mut ed = editor::editor::Editor::new(std::env::current_dir().unwrap());
    let mut buf = Buffer::from_str(&src);
    buf.path = Some("big.rs".into());
    ed.docs.push(Doc::new(buf));
    ed.rows = 40;
    ed.set_cursor(Pos::new(1000, 0));
    ed.scroll_to_cursor();
    let (best, frame) = best_of(5, || nsh_app_editor::model::frame(&ed, 40));
    eprintln!("the frame of a 40-line window at line 1000: {best:.2} ms ({} rows)", frame.rows.len());
    budget("a 40-line frame", best, 50.0);
}

#[test]
#[ignore]
fn bench_finder_and_tree() {
    // 200,000 paths shaped like a large monorepo.
    let mut files = Vec::new();
    for a in 0..50 {
        for b in 0..40 {
            for c in 0..100 {
                files.push(format!("packages/pkg{a}/src/module{b}/component_{c}_view.tsx"));
            }
        }
    }
    files.sort();
    for q in ["comp", "pkg12view", "m3c7v", "zzz"] {
        let (best, (m, total)) = best_of(5, || nsh_app_editor::fuzzy::rank(q, &files, 200));
        eprintln!("fuzzy rank {q:>10} over {} files: {best:.1} ms ({total} match, {} kept)", files.len(), m.len());
        budget("fuzzy rank", best, 500.0);
    }
    let (best, idx) = best_of(5, || nsh_app_editor::app::tree::TreeIndex::build(&files));
    eprintln!("tree index build: {best:.1} ms");
    budget("tree index build", best, 1000.0);
    let mut open = std::collections::HashSet::new();
    for a in 0..50 {
        open.insert(format!("packages/pkg{a}"));
        open.insert(format!("packages/pkg{a}/src"));
    }
    open.insert("packages".to_string());
    let (best, rows) = best_of(5, || idx.rows(&open));
    eprintln!("tree rows with 100 folders open: {best:.2} ms ({} rows)", rows.len());
    budget("tree rows", best, 50.0);
}

/// Keys in a 200,000-line file, each from the press until the app is quiet.
#[test]
#[ignore]
fn bench_big_file_keys() {
    let src = big_source();
    let dir = scratch();
    let file = write(dir.path(), "big.rs", &src);
    let t = Instant::now();
    let buf = Buffer::from_str(&src);
    eprintln!("big file: {} lines, {:.1} MB, buffer built in {:.1} ms", buf.line_count(), src.len() as f64 / 1e6, ms(t));
    let t = Instant::now();
    let mut f = flow(&file, Some(&keymap_config("neovim")));
    eprintln!("open until the buffer is active: {:.1} ms", ms(t));
    let mut worst: f64 = 0.0;
    let mut note = |what: &str, took: f64| {
        eprintln!("{what:<26}{took:.2} ms");
        worst = worst.max(took);
    };
    note("j (one line down):", keys(&mut f, "j"));
    note("G (to the end):", keys(&mut f, "G"));
    note("gg (to the start):", keys(&mut f, "gg"));
    note("100G:", keys(&mut f, "100G"));
    note("Ctrl-d (half page):", stroke(&mut f, "ctrl+d"));
    for k in ["i", "\x1b", "a", "\x1b", "x", "o", "\x1b"] {
        note(&format!("{k:?}:"), keys(&mut f, k));
    }
    let per: Vec<String> = "olet ans".chars().map(|c| format!("{c}={:.1}", keys(&mut f, &c.to_string()))).collect();
    eprintln!("o + type, each key: {}", per.join(" "));
    note("o + type 20 chars, worst:", keys(&mut f, "wer = 42; // x"));
    note("Esc:", keys(&mut f, "\x1b"));
    note("u (undo):", keys(&mut f, "u"));
    note("dd:", keys(&mut f, "dd"));
    note("/needle + Enter (worst):", keys(&mut f, "/fn poll\n"));
    note("n (next match):", keys(&mut f, "n"));
    note("* (word under cursor):", keys(&mut f, "*"));
    let t = Instant::now();
    keys(&mut f, ":%s/self/this/g\n");
    let subst = ms(t);
    eprintln!(":%s over every line:      {subst:.1} ms");
    note("u after :%s:", keys(&mut f, "u"));
    note("G then type at end, worst", keys(&mut f, "Goend\x1b"));
    budget("the worst key in a big file", worst, 500.0);
    budget(":%s over a big file", subst, 10_000.0);
}

#[test]
#[ignore]
fn bench_first_edit_in_core() {
    let src = big_source();
    let mut buf = Buffer::from_str(&src);
    for round in 0..3 {
        let t = Instant::now();
        buf.begin(Pos::new(0, 0));
        buf.insert_at(5, "x");
        buf.commit(Pos::new(0, 1));
        let took = ms(t);
        eprintln!("core edit {round}: {took:.3} ms");
        budget("a core edit", took, 50.0);
    }
    let mut ed = editor::editor::Editor::new(std::env::current_dir().unwrap());
    ed.docs.push(Doc::new(Buffer::from_str(&src)));
    for (round, c) in ['x', 'x', 'i', 'a', '\u{1b}'].into_iter().enumerate() {
        let k = if c == '\u{1b}' { editor::editor::KeyInput::plain(editor::editor::KeyCode::Esc) } else { editor::editor::KeyInput::ch(c) };
        let t = Instant::now();
        editor::vim::handle_key(&mut ed, k);
        let took = ms(t);
        eprintln!("vim key {round} {c:?}: {took:.3} ms");
        budget("a vim key in the core", took, 50.0);
    }
}

#[test]
#[ignore]
fn bench_first_edit_small_file() {
    let dir = scratch();
    let file = write(dir.path(), "small.rs", "fn main() {\n    let a = 1;\n}\n");
    let t = Instant::now();
    let mut f = flow(&file, Some(&keymap_config("neovim")));
    eprintln!("small file open: {:.2} ms", ms(t));
    for k in ["x", "x", "u", "u", "x"] {
        let took = keys(&mut f, k);
        eprintln!("small file {k}: {took:.2} ms");
        budget("a key in a small file", took, 200.0);
    }
}

#[test]
#[ignore]
fn bench_index_walk() {
    let root = std::env::var("BENCH_WALK").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string());
    let root = std::path::PathBuf::from(root);
    let (best, files) = best_of(3, || nsh_app_editor::services::index::walk(&root));
    eprintln!("index walk of {}: {best:.1} ms ({} files)", root.display(), files.len());
    // The walk on one thread, as it was, for comparison.
    let (one, single) = best_of(3, || {
        let mut v: Vec<String> = ignore::WalkBuilder::new(&root)
            .hidden(false)
            .require_git(false)
            .filter_entry(|e| e.file_name() != ".git")
            .build()
            .flatten()
            .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
            .filter_map(|e| e.path().strip_prefix(&root).ok().map(|r| r.to_string_lossy().into_owned()))
            .collect();
        v.sort();
        v
    });
    eprintln!("index walk on one thread: {one:.1} ms ({} files)", single.len());
    assert_eq!(files.len(), single.len().min(200_000));
    budget("the index walk", best, 5000.0);
}

#[test]
#[ignore]
fn bench_substitute_and_search() {
    let src = big_source();
    let fresh = || {
        let mut ed = editor::editor::Editor::new(std::env::current_dir().unwrap());
        ed.docs.push(Doc::new(Buffer::from_str(&src)));
        ed
    };
    let mut ed = fresh();
    let (best, _) = best_of(3, || editor::vim::run_ex(&mut ed, "g/zzzz_not_here/d"));
    eprintln!(":g with no match over {} lines: {best:.1} ms", ed.buf().line_count());
    budget(":g with no match", best, 2000.0);
    for cmd in ["%s/self/this/g", "%s/\\<fn\\>/fun/g", "%s/x*y//g"] {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let mut ed = fresh();
            let t = Instant::now();
            editor::vim::run_ex(&mut ed, cmd);
            best = best.min(ms(t));
        }
        eprintln!(":{cmd} over 200k lines: {best:.1} ms");
        budget(cmd, best, 10_000.0);
    }
}
