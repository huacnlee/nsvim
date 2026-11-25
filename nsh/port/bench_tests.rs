//! Timings for performance work: `cargo test --release --bin editor bench_ -- --ignored --nocapture`.

use std::time::Instant;

use editor::buffer::Buffer;
use editor::editor::Doc;
use rune::{Env, Runtime};

use super::{App, DocMeta};
use crate::config::Config;

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

fn app_with(family: &'static str) -> App {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut app = App::new(Config::default(), root.clone(), false, None);
    let path = root.join("src/editor.rs");
    let buf = Buffer::open(&path).unwrap();
    app.meta.insert(path, DocMeta { lang: rune_highlights::Lang::from_name("rust"), ..Default::default() });
    app.ed.docs.push(Doc::new(buf));
    app.font_family = family;
    app
}

#[test]
#[ignore]
fn bench_first_frame() {
    let order = std::env::var("BENCH_ORDER").unwrap_or_default();
    let mut list = vec![
        ("configured list (with ui-monospace)", crate::ui::font::family("JetBrains Mono")),
        ("named fonts only", "\"JetBrains Mono\", \"JetBrainsMono Nerd Font\""),
        ("configured list, again", crate::ui::font::family("JetBrains Mono")),
    ];
    if order == "named" {
        list.swap(0, 1);
    }
    for (name, fam) in list {
        let t = Instant::now();
        let mut rt = Runtime::new(app_with(fam), crate::ui::view, (1440.0, 900.0), Env { system_dark: true, scale: 2.0 });
        let made = ms(t);
        let t = Instant::now();
        rt.frame(0.0);
        let first = ms(t);
        let t = Instant::now();
        rt.frame(1.0 / 60.0);
        let second = ms(t);
        eprintln!("{name}: runtime {made:.1} ms, frame 1 {first:.1} ms, frame 2 {second:.1} ms");
    }
}

#[test]
#[ignore]
fn bench_highlight_window() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap().repeat(4);
    let buf = Buffer::from_str(&src);
    let lang = rune_highlights::Lang::from_name("rust").unwrap();
    let t = Instant::now();
    let r = crate::highlight::runs(42, &buf, lang, 1000, 40);
    eprintln!("highlight a 40-line window at line 1000: {:.2} ms ({} lines)", ms(t), r.len());
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
        let (best, (m, total)) = best_of(5, || crate::fuzzy::rank(q, &files, 200));
        eprintln!("fuzzy rank {q:>10} over {} files: {best:.1} ms ({total} match, {} kept)", files.len(), m.len());
    }
    let (best, idx) = best_of(5, || super::tree::TreeIndex::build(&files));
    eprintln!("tree index build: {best:.1} ms");
    let mut open = std::collections::HashSet::new();
    for a in 0..50 {
        open.insert(format!("packages/pkg{a}"));
        open.insert(format!("packages/pkg{a}/src"));
    }
    open.insert("packages".to_string());
    let (best, rows) = best_of(5, || idx.rows(&open));
    eprintln!("tree rows with 100 folders open: {best:.2} ms ({} rows)", rows.len());
}

fn key(rt: &mut Runtime<App>, k: rune::Key, ctrl: bool) -> f64 {
    let t = Instant::now();
    rt.key(rune::KeyEvent { key: k, mods: rune::Mods { ctrl, ..Default::default() } });
    rt.frame(1.0 / 60.0);
    ms(t)
}

fn keys(rt: &mut Runtime<App>, s: &str) -> f64 {
    let mut worst: f64 = 0.0;
    for c in s.chars() {
        let k = match c {
            '\n' => rune::Key::Enter,
            '\x1b' => rune::Key::Escape,
            c => rune::Key::Text(c.to_string()),
        };
        worst = worst.max(key(rt, k, false));
    }
    worst
}

#[test]
#[ignore]
fn bench_big_file_keys() {
    let one = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap();
    let src = one.repeat(200_000 / one.lines().count() + 1);
    let root = std::env::temp_dir();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    let path = root.join("editor-bench-big.rs");
    let t = Instant::now();
    let mut buf = Buffer::from_str(&src);
    eprintln!("big file: {} lines, {:.1} MB, buffer built in {:.1} ms", buf.line_count(), src.len() as f64 / 1e6, ms(t));
    buf.path = Some(path.clone());
    app.meta.insert(path, DocMeta { lang: rune_highlights::Lang::from_name("rust"), ..Default::default() });
    app.ed.docs.push(Doc::new(buf));
    let mut rt = Runtime::new(app, crate::ui::view, (1440.0, 900.0), Env { system_dark: true, scale: 2.0 });
    rt.on_key = Some(super::keys::on_key);
    rt.use_clipboard(&rune::clipboard::memory());
    let t = Instant::now();
    rt.frame(0.0);
    // The app's second poll turns on the glyph warm-up. Tests have no poll, so set it here.
    rt.update(|s: &mut App, _| s.polls = 2);
    rt.frame(1.0 / 60.0);
    eprintln!("first two frames: {:.1} ms", ms(t));
    let mut idle = Vec::new();
    for _ in 0..20 {
        let t = Instant::now();
        rt.frame(1.0 / 60.0);
        idle.push(ms(t));
    }
    eprintln!("idle frame: {:.3} ms", idle.iter().sum::<f64>() / idle.len() as f64);
    eprintln!("j (one line down):        {:.2} ms", keys(&mut rt, "j"));
    eprintln!("G (to the end):           {:.2} ms", keys(&mut rt, "G"));
    eprintln!("gg (to the start):        {:.2} ms", keys(&mut rt, "gg"));
    eprintln!("100G:                     {:.2} ms", keys(&mut rt, "100G"));
    eprintln!("Ctrl-d (half page):       {:.2} ms", key(&mut rt, rune::Key::Command('d'), true));
    for k in ["i", "\x1b", "a", "\x1b", "x", "o", "\x1b"] {
        let key = if k == "\x1b" { rune::Key::Escape } else { rune::Key::Text(k.into()) };
        let t = Instant::now();
        rt.key(rune::KeyEvent { key, mods: rune::Mods::default() });
        let a = ms(t);
        let t = Instant::now();
        rt.frame(1.0 / 60.0);
        eprintln!("split {k:?}: key {a:.2} ms, frame {:.2} ms", ms(t));
    }
    let per: Vec<String> = "olet ans".chars().map(|c| format!("{c}={:.1}", keys(&mut rt, &c.to_string()))).collect();
    eprintln!("o + type, each key: {}", per.join(" "));
    eprintln!("o + type 20 chars, worst: {:.2} ms", keys(&mut rt, "wer = 42; // x"));
    eprintln!("Esc:                      {:.2} ms", keys(&mut rt, "\x1b"));
    eprintln!("u (undo):                 {:.2} ms", keys(&mut rt, "u"));
    eprintln!("dd:                       {:.2} ms", keys(&mut rt, "dd"));
    eprintln!("/needle + Enter (worst):  {:.2} ms", keys(&mut rt, "/fn poll\n"));
    eprintln!("n (next match):           {:.2} ms", keys(&mut rt, "n"));
    eprintln!("* (word under cursor):    {:.2} ms", keys(&mut rt, "*"));
    let t = Instant::now();
    keys(&mut rt, ":%s/self/this/g\n");
    eprintln!(":%s over every line:      {:.1} ms", ms(t));
    eprintln!("u after :%s:              {:.1} ms", keys(&mut rt, "u"));
    eprintln!("G then type at end, worst {:.2} ms", keys(&mut rt, "Goend\x1b"));
}

#[test]
#[ignore]
fn bench_first_edit_in_core() {
    let one = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap();
    let src = one.repeat(200_000 / one.lines().count() + 1);
    let mut buf = Buffer::from_str(&src);
    for round in 0..3 {
        let t = Instant::now();
        buf.begin(editor::buffer::Pos::new(0, 0));
        buf.insert_at(5, "x");
        buf.commit(editor::buffer::Pos::new(0, 1));
        eprintln!("core edit {round}: {:.3} ms", ms(t));
    }
    let mut ed = editor::editor::Editor::new(std::env::temp_dir());
    ed.docs.push(Doc::new(Buffer::from_str(&src)));
    for (round, c) in ['x', 'x', 'i', 'a', '\u{1b}'].into_iter().enumerate() {
        let k = if c == '\u{1b}' { editor::editor::KeyInput::plain(editor::editor::KeyCode::Esc) } else { editor::editor::KeyInput::ch(c) };
        let t = Instant::now();
        editor::vim::handle_key(&mut ed, k);
        eprintln!("vim key {round} {c:?}: {:.3} ms", ms(t));
    }
}

#[test]
#[ignore]
fn bench_first_edit_small_file() {
    let root = std::env::temp_dir();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    let path = root.join("editor-bench-small.rs");
    let mut buf = Buffer::from_str("fn main() {\n    let a = 1;\n}\n");
    buf.path = Some(path.clone());
    app.meta.insert(path, DocMeta { lang: rune_highlights::Lang::from_name("rust"), ..Default::default() });
    app.ed.docs.push(Doc::new(buf));
    let mut rt = Runtime::new(app, crate::ui::view, (1440.0, 900.0), Env { system_dark: true, scale: 2.0 });
    rt.on_key = Some(super::keys::on_key);
    let t = Instant::now();
    rt.frame(0.0);
    eprintln!("small file frame 1: {:.2} ms", ms(t));
    // The app's second poll turns on the glyph warm-up. Tests have no poll, so set it here.
    rt.update(|s: &mut App, _| s.polls = 2);
    let t = Instant::now();
    rt.frame(1.0 / 60.0);
    eprintln!("small file frame 2 (glyph warm-up): {:.2} ms", ms(t));
    for k in ["x", "x", "u", "u", "x"] {
        eprintln!("small file {k}: {:.2} ms", keys(&mut rt, k));
    }
}

#[test]
#[ignore]
fn bench_glyph_fallback() {
    struct S {
        s: String,
        fam: &'static str,
    }
    fn view(s: &S, _: &mut rune::Cx<S>) -> rune::El {
        rune::text(s.s.clone()).font(rune::tok::TextStyle::new(13.0, 400.0).family(rune::Family::Named(s.fam)))
    }
    for fam in ["Inter", crate::ui::font::family("JetBrains Mono")] {
        let mut rt = Runtime::new(S { s: "abc".into(), fam }, view, (800.0, 200.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        rt.frame(1.0 / 60.0);
        for g in ["x", "●", "×", "[+]", "⇄", "Δ", "┃", "◆", "⋯", "…", "⎇", "·", "▸", "✓"] {
            let s = format!("abc {g}");
            rt.update(move |st: &mut S, _| st.s = s);
            let t = Instant::now();
            rt.frame(1.0 / 60.0);
            eprintln!("glyph {fam:.20} {g}: {:.2} ms", ms(t));
        }
    }
}

#[test]
#[ignore]
fn bench_first_frame_after_measure_warm() {
    let fam = crate::ui::font::family("JetBrains Mono");
    let t = Instant::now();
    let style = rune::tok::TextStyle::new(13.0, 400.0).family(rune::Family::Named(fam));
    let size = rune::text::measure("warm", &[], style, None);
    eprintln!("warm measure: {:.1} ms {size:?}", ms(t));
    let t = Instant::now();
    let ui = rune::tok::TextStyle::new(13.0, 400.0);
    rune::text::measure("warm", &[], ui, None);
    eprintln!("warm measure ui: {:.1} ms", ms(t));
    let mut rt = Runtime::new(app_with(fam), crate::ui::view, (1440.0, 900.0), Env { system_dark: true, scale: 2.0 });
    let t = Instant::now();
    rt.frame(0.0);
    eprintln!("frame 1 after the warm measure: {:.1} ms", ms(t));
}

#[test]
#[ignore]
fn bench_index_walk() {
    let root = std::env::var("BENCH_WALK").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR")).to_string());
    let root = std::path::PathBuf::from(root);
    let (best, files) = best_of(3, || crate::services::index::walk(&root));
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
}

#[test]
#[ignore]
fn bench_substitute_and_search() {
    let one = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs")).unwrap();
    let src = one.repeat(200_000 / one.lines().count() + 1);
    let fresh = || {
        let mut ed = editor::editor::Editor::new(std::env::temp_dir());
        ed.docs.push(Doc::new(Buffer::from_str(&src)));
        ed
    };
    let mut ed = fresh();
    let (best, _) = best_of(3, || editor::vim::run_ex(&mut ed, "g/zzzz_not_here/d"));
    eprintln!(":g with no match over {} lines: {best:.1} ms", ed.buf().line_count());
    for cmd in ["%s/self/this/g", "%s/\\<fn\\>/fun/g", "%s/x*y//g"] {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let mut ed = fresh();
            let t = Instant::now();
            editor::vim::run_ex(&mut ed, cmd);
            best = best.min(ms(t));
        }
        eprintln!(":{cmd} over 200k lines: {best:.1} ms");
    }
}
