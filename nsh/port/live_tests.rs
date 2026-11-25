//! The leader menu, the command palette and the fixes from the key audit, run against the real
//! app with its real workers: git, the file watcher, ripgrep and clangd.

use super::*;

/// A C file in git: committed, then changed on lines 4 and 5 and on line 15. `main` calls `twice`, and
/// `broken` holds an error for the language server to report.
const LEADER_C: &str = "#include <stdio.h>

static int twice(int n) {
    int base = 1;
    return n * 2 * base;
}

/* One. */
/* Two. */
/* Three. */
/* Four. */
int main(void) {
    int total = twice(21);
    printf(\"%d\\n\", total);
    return total;
}

int broken(void) { return missing_name; }
";

/// Every key the leader menu shows, pressed in the real app with its real workers: git, the
/// file watcher, ripgrep and clangd. Each one must do what its label says. The LSP keys run
/// when clangd is installed.
#[test]
fn every_leader_menu_key_does_what_it_says() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").args(args).current_dir(&root).output().expect("git");
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "test"]);
    std::fs::write(root.join("main.c"), LEADER_C).unwrap();
    std::fs::write(root.join("notes.txt"), "a needle in the notes\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let changed = LEADER_C
        .replace("int base = 1;", "int base = 2;")
        .replace("return n * 2 * base;", "return n * 2 * base * 1;")
        .replace("return total;", "return total + 0;");
    std::fs::write(root.join("main.c"), &changed).unwrap();
    let main_c = root.join("main.c");

    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    let open = main_c.clone();
    rt.update(move |s: &mut App, _| s.open_path(open, None));
    settle(&mut rt, "open main.c", |s| s.ed.has_doc() && s.ed.buf().path.as_deref() == Some(main_c.as_path()));
    let lsp = std::process::Command::new("clangd").arg("--version").output().is_ok();

    // Top level.
    leader(&mut rt, "f");
    settle(&mut rt, "SPC f opens the file finder", |s| picking(s, PickKind::Files));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "b");
    settle(&mut rt, "SPC b lists buffers", |s| picking(s, PickKind::Buffers));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "/");
    settle(&mut rt, "SPC / opens project search", |s| picking(s, PickKind::Grep));
    typ(&mut rt, "needle");
    settle(&mut rt, "SPC / finds the needle", |s| !s.grep.hits.is_empty());
    typ(&mut rt, "\x1b");
    leader(&mut rt, "e");
    settle(&mut rt, "SPC e focuses the tree", |s| s.tree.show && s.tree.focus);
    typ(&mut rt, "\x1b");
    settle(&mut rt, "Esc leaves the tree", |s| !s.tree.focus);
    leader(&mut rt, ",");
    settle(&mut rt, "SPC , opens settings", |s| s.on_settings);
    rt.update(|s: &mut App, _| s.on_settings = false);
    leader(&mut rt, "?");
    settle(&mut rt, "SPC ? shows every key", |s| picking(s, PickKind::Keys));
    typ(&mut rt, "\x1b");

    // +git: the two changed blocks, in order and around.
    settle(&mut rt, "git signs", |s| s.meta.get(&s.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.signs.is_empty()));
    // From the line just above a two-line change: to its first line, then past its second.
    rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(2, 0)));
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n goes to the first change", |s| s.ed.cursor().line == 3);
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n goes to the next change, not the next line", |s| s.ed.cursor().line == 14);
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n wraps to the first change", |s| s.ed.cursor().line == 3);
    leader(&mut rt, "gp");
    settle(&mut rt, "SPC g p wraps to the last change", |s| s.ed.cursor().line == 14);
    leader(&mut rt, "gs");
    settle(&mut rt, "SPC g s focuses the tree", |s| s.tree.focus);
    typ(&mut rt, "\x1b");

    // +lsp, on `twice` where main calls it.
    if lsp {
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        settle(&mut rt, "clangd reports the error", |s| s.meta.get(&s.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.diags.is_empty()));
        leader(&mut rt, "d");
        settle(&mut rt, "SPC d lists the error clangd reports", |s| {
            picking(s, PickKind::Locations) && s.locations.iter().any(|l| l.text.contains("missing_name"))
        });
        typ(&mut rt, "\x1b");
        leader(&mut rt, "lh");
        settle(&mut rt, "SPC l h shows hover", |s| s.hover.as_deref().is_some_and(|h| h.contains("twice")));
        typ(&mut rt, "\x1b");
        leader(&mut rt, "lr");
        settle(&mut rt, "SPC l r lists references", |s| picking(s, PickKind::Locations) && s.locations.len() >= 2);
        typ(&mut rt, "\x1b");
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        leader(&mut rt, "la");
        settle(&mut rt, "SPC l a answers with code actions", |s| picking(s, PickKind::Actions) || said(s, "no code actions"));
        if picking_now(&mut rt) {
            typ(&mut rt, "\x1b");
        }
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        leader(&mut rt, "ld");
        settle(&mut rt, "SPC l d goes to the definition", |s| s.ed.cursor().line == 2);
    } else {
        eprintln!("clangd is not installed: the SPC l and SPC d checks did not run");
    }
    rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(2, 11)));
    leader(&mut rt, "ln");
    settle(&mut rt, "SPC l n opens :Rename", |s| s.ed.cmd.as_ref().is_some_and(|c| c.text == "Rename twice"));
    typ(&mut rt, "\x1b");

    // SPC w writes; then an outside change is marked, listed and cleared.
    typ(&mut rt, "ggx");
    leader(&mut rt, "w");
    settle(&mut rt, "SPC w writes the file", |s| !s.ed.buf().is_dirty());
    std::thread::sleep(std::time::Duration::from_millis(400));
    std::fs::write(root.join("main.c"), format!("{changed}/* From outside. */\n")).unwrap();
    settle(&mut rt, "the outside change is marked", |s| !s.outside.is_empty());
    leader(&mut rt, "al");
    settle(&mut rt, "SPC a l lists outside changes", |s| picking(s, PickKind::Locations));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "ac");
    settle(&mut rt, "SPC a c clears the marks", |s| s.outside.is_empty() && said(s, "cleared"));

    // +agent: notes and proposals.
    leader(&mut rt, "ap");
    settle(&mut rt, "SPC a p with none says so", |s| said(s, "no proposals"));
    let r = ask(&mut rt, "reveal", serde_json::json!({"path": "main.c", "line": 3, "end_line": 5, "note": "look"}));
    assert!(r.ok, "{:?}", r.error);
    settle(&mut rt, "the note shows", |s| s.note.is_some());
    leader(&mut rt, "an");
    settle(&mut rt, "SPC a n clears the note", |s| s.note.is_none());
    let notes = root.join("notes.txt");
    rt.update(move |s: &mut App, _| s.open_path(notes, None));
    settle(&mut rt, "open notes.txt", |s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("notes.txt")));
    let r = ask(&mut rt, "propose", serde_json::json!({"path": "main.c", "text": "int main(void) { return 0; }\n", "label": "smaller"}));
    assert!(r.ok, "{:?}", r.error);
    leader(&mut rt, "ap");
    settle(&mut rt, "SPC a p shows the proposal", |s| {
        s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("main.c")) && s.proposal_here().is_some()
    });
    typ(&mut rt, "n");

    // SPC q closes the buffer.
    let before = rt.update_if(|s: &mut App, _| s.ed.docs.len() == 2);
    assert!(before, "two buffers open");
    leader(&mut rt, "q");
    settle(&mut rt, "SPC q closes the buffer", |s| s.ed.docs.len() == 1);

    // A key added to the menu must be added here too.
    let tested = ["f", "b", "/", "e", ",", "?", "d", "gn", "gp", "gs", "ld", "lr", "lh", "ln", "la", "w", "al", "ac", "ap", "an", "q"];
    let shown = crate::app::keys::LEADER.iter().filter(|(_, _, group)| !group).map(|(c, _, _)| c.to_string());
    let groups = [('g', crate::app::keys::LEADER_GIT), ('l', crate::app::keys::LEADER_LSP), ('a', crate::app::keys::LEADER_AGENT)];
    let shown = shown.chain(groups.iter().flat_map(|(g, keys)| keys.iter().map(move |(c, _, _)| format!("{g}{c}"))));
    for key in shown {
        assert!(tested.contains(&key.as_str()), "SPC {key} shows in the menu but this test does not press it");
    }
}

/// Every entry of the command palette, run in the real app with its real workers: each one must
/// do what its label says, or say why it cannot.
#[test]
fn every_palette_command_does_what_it_says() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "alpha\n").unwrap();
    std::fs::write(root.join("b.txt"), "beta\n").unwrap();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    for f in ["a.txt", "b.txt"] {
        let path = root.join(f);
        rt.update(move |s: &mut App, _| s.open_path(path, None));
        settle(&mut rt, "open", |s| s.ed.has_doc() && s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with(f)));
    }
    let cmd = |rt: &mut Runtime<App>, id: &'static str| rt.update(move |s: &mut App, t| s.run_command(id, t));
    let mut tested = Vec::new();
    let mut check = |rt: &mut Runtime<App>, id: &'static str, what: &str, done: &dyn Fn(&App) -> bool| {
        cmd(rt, id);
        // A result the command sets at once counts before later events can replace it.
        if !done(&rt.state) {
            settle(rt, what, done);
        }
        tested.push(id);
        typ(rt, "\x1b");
    };
    check(&mut rt, "find", "find opens the finder", &|s| picking(s, PickKind::Files));
    check(&mut rt, "grep", "grep opens project search", &|s| picking(s, PickKind::Grep));
    typ(&mut rt, "x");
    check(&mut rt, "save", "save writes the file", &|s| !s.ed.buf().is_dirty());
    typ(&mut rt, "x");
    check(&mut rt, "save-all", "save all writes every file", &|s| s.ed.docs.iter().all(|d| !d.buf.is_dirty()));
    let shown = rt.update_if(|s: &mut App, _| s.tree.show);
    check(&mut rt, "tree", "tree toggles the file tree", &move |s| s.tree.show != shown);
    check(&mut rt, "settings", "settings opens settings", &|s| s.on_settings);
    // Close the Settings tab, so next and prev cycle the two files only.
    rt.update(|s: &mut App, _| {
        s.on_settings = false;
        s.settings = None;
    });
    check(&mut rt, "next", "next goes to the next buffer", &|s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("a.txt")));
    check(&mut rt, "prev", "prev goes to the previous buffer", &|s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("b.txt")));
    check(&mut rt, "keymap=classic", "the classic keymap", &|s| s.ed.keymap == Keymap::Classic);
    check(&mut rt, "keymap=neovim", "the Neovim keymap", &|s| s.ed.keymap == Keymap::Vim);
    for (id, mode) in [("theme=light", "light"), ("theme=system", "system"), ("theme=omarchy", "omarchy"), ("theme=dark", "dark")] {
        check(&mut rt, id, id, &move |s| s.cfg.theme.mode == mode);
    }
    let wrap = rt.update_if(|s: &mut App, _| s.cfg.editor.wrap);
    check(&mut rt, "wrap", "wrap toggles soft wrap", &move |s| s.cfg.editor.wrap != wrap);
    check(&mut rt, "keys", "keys shows every key", &|s| picking(s, PickKind::Keys));
    check(&mut rt, "outside", "outside says when there is nothing", &|s| said(s, "no changes from outside"));
    check(&mut rt, "clear-outside", "clear-outside confirms", &|s| said(s, "cleared"));
    check(&mut rt, "proposal", "proposal says when there is none", &|s| said(s, "no proposals"));
    typ(&mut rt, "x");
    check(&mut rt, "quit", "quit refuses to drop unsaved edits", &|s| s.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("E37")));
    typ(&mut rt, "u");
    check(&mut rt, "close", "close closes the buffer", &|s| s.ed.docs.len() == 1);

    // `:set theme=` takes a theme id too.
    typ(&mut rt, ":set theme=gruvbox\n");
    settle(&mut rt, ":set theme=gruvbox", |s| s.cfg.theme.mode == "dark" && s.cfg.theme.dark == "gruvbox");
    typ(&mut rt, ":set theme=flexoki-light\n");
    settle(&mut rt, ":set theme=flexoki-light", |s| s.cfg.theme.mode == "light" && s.cfg.theme.light == "flexoki-light");
    typ(&mut rt, ":set theme=nope\n");
    settle(&mut rt, ":set theme=nope is an error", |s| s.ed.msg.as_ref().is_some_and(|m| m.error));

    // An entry added to the palette must be added here too.
    for (label, id) in crate::app::picker::COMMANDS {
        assert!(tested.contains(id), "\"{label}\" ({id}) shows in the palette but this test does not run it");
    }
}

/// Fixes from the key audit, in the app: the tree, Settings, `:set`, `Ctrl r +` and `:wqa`.
#[test]
fn the_tree_keeps_its_keys_and_passes_ctrl_shortcuts_on() {
    let mut app = app("classic");
    app.files = vec!["a.rs".into(), "b.rs".into()];
    app.tree.index = crate::app::tree::TreeIndex::build(&app.files);
    app.tree.show = true;
    app.tree.focus = true;
    let mut rt = run(app);
    let before = text(&rt);
    typ(&mut rt, "x");
    assert_eq!(text(&rt), before, "a plain key the tree does not use never edits the file");
    assert!(rt.state.tree.focus);
    ctrl(&mut rt, 'p');
    assert!(picking(&rt.state, PickKind::Files), "Ctrl P finds a file while the tree has focus");
}

#[test]
fn settings_take_the_classic_window_shortcuts_and_wq_closes_them() {
    let mut rt = run(app("classic"));
    let mut b = Buffer::from_str("two\n");
    b.path = Some(std::env::temp_dir().join("editor-test-two.rs"));
    rt.update(move |s: &mut App, _| s.ed.docs.push(Doc::new(b)));
    rt.update(|s: &mut App, _| s.open_settings());
    ctrl(&mut rt, 'p');
    assert!(picking(&rt.state, PickKind::Files), "Ctrl P works in Settings");
    typ(&mut rt, "\x1b");
    press(&mut rt, Key::Tab, Mods { ctrl: true, cmd: true, ..Default::default() });
    assert!(!rt.state.on_settings, "Ctrl Tab goes to the next tab, not the next section");
    let mut rt = run(app("neovim"));
    rt.update(|s: &mut App, _| s.open_settings());
    typ(&mut rt, ":wq\n");
    assert!(!rt.state.on_settings && rt.state.settings.is_none(), ":wq writes and closes Settings");
}

#[test]
fn set_shows_and_flips_app_options() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, ":set nu?\n");
    assert!(said(&rt.state, "number=true"), "{:?}", rt.state.ed.msg);
    typ(&mut rt, ":set nu!\n");
    assert_eq!(rt.state.cfg.editor.numbers, "off");
    typ(&mut rt, ":set cul!\n");
    assert!(!rt.state.cfg.editor.cursorline);
    for line in [":set hlsearch\n", ":set ic\n", ":set noscs\n", ":set noligatures\n", ":set sts=2\n"] {
        typ(&mut rt, line);
        assert!(!rt.state.ed.msg.as_ref().is_some_and(|m| m.error), "{line}: {:?}", rt.state.ed.msg);
    }
    assert!(!rt.state.cfg.font.ligatures);
    assert_eq!(rt.state.cfg.indent.width, 2);
    typ(&mut rt, ":set nope?\n");
    assert!(rt.state.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("E518")));
}

#[test]
fn ctrl_r_plus_types_the_system_clipboard() {
    let mut rt = run(app("neovim"));
    let clip = rune::clipboard::memory();
    rt.use_clipboard(&clip);
    clip.set_text("from the clipboard ");
    typ(&mut rt, "i");
    ctrl(&mut rt, 'r');
    typ(&mut rt, "+");
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    typ(&mut rt, "\x1b");
    assert!(text(&rt).starts_with("from the clipboard fn main()"), "{}", text(&rt));
}

#[test]
fn wqa_writes_every_buffer_before_it_quits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "alpha\n").unwrap();
    std::fs::write(root.join("b.txt"), "beta\n").unwrap();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    for f in ["a.txt", "b.txt"] {
        let path = root.join(f);
        rt.update(move |s: &mut App, _| s.open_path(path, None));
        settle(&mut rt, "open", |s| s.ed.has_doc() && s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with(f)));
        typ(&mut rt, "x");
    }
    typ(&mut rt, ":wqa\n");
    assert!(!rt.state.ed.msg.as_ref().is_some_and(|m| m.error), "{:?}", rt.state.ed.msg);
    assert_eq!(rt.state.quit_after_saves, Some(2), "it waits for both writes before it quits");
    // Not polled further: the last write would quit, and a quit saves the session to disk.
    for (f, want) in [("a.txt", "lpha\n"), ("b.txt", "eta\n")] {
        let start = std::time::Instant::now();
        while std::fs::read_to_string(root.join(f)).unwrap() != want {
            assert!(start.elapsed().as_secs() < 10, "{f} was not written");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

/// The file watcher reads back the editor's own save. Typing right after `:w` makes the buffer
/// differ from that read-back, which is not a change from outside.
#[test]
fn typing_right_after_a_save_is_not_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "alpha\n").unwrap();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    let path = root.join("a.txt");
    let open = path.clone();
    rt.update(move |s: &mut App, _| s.open_path(open, None));
    settle(&mut rt, "open", |s| s.ed.has_doc());
    typ(&mut rt, "x:w\n");
    settle(&mut rt, "the save", |s| !s.ed.buf().is_dirty());
    typ(&mut rt, "x");
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < 1500 {
        let clock = 100.0 + start.elapsed().as_secs_f64();
        rt.update(move |s: &mut App, t| {
            s.poll(clock, t);
        });
        rt.frame(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!rt.state.meta.get(&path).is_some_and(|m| m.conflict), "marked as changed on disk");
    assert!(!said(&rt.state, "W12"), "{:?}", rt.state.ed.msg);
}

fn key_rows(s: &App) -> Vec<String> {
    let Overlay::Pick(p) = &s.overlay else { return Vec::new() };
    p.matches.iter().map(|m| p.items[m.index].clone()).collect()
}

/// `F1`, `SPC ?`, `:Keys`, `:h` and the tree's `?` show every key, and the search keeps the rows
/// with every word. Each keymap lists its own keys.
#[test]
fn every_key_shows_in_a_list_with_search() {
    let mut rt = run(super::app("neovim"));
    let f1 = |rt: &mut Runtime<App>| press(rt, Key::Function(1), Mods::default());
    f1(&mut rt);
    assert!(picking(&rt.state, PickKind::Keys), "F1 opens the list");
    let all = key_rows(&rt.state);
    assert!(
        all.len() > 300 && all.iter().any(|r| r.starts_with("SPC f ")) && !all.iter().any(|r| r.starts_with("Ctrl D ")),
        "{}",
        all.len()
    );
    typ(&mut rt, "tree");
    let found = key_rows(&rt.state);
    assert!(!found.is_empty() && found.len() < 60, "{} rows", found.len());
    assert!(found.iter().all(|r| r.to_lowercase().contains("tree")), "{found:#?}");
    f1(&mut rt);
    assert!(matches!(rt.state.overlay, Overlay::None), "F1 again closes it");

    typ(&mut rt, ":Keys dd\n");
    assert!(picking(&rt.state, PickKind::Keys), ":Keys");
    let found = key_rows(&rt.state);
    assert!(!found.is_empty() && found.iter().all(|r| r.contains("dd")), "{found:#?}");
    typ(&mut rt, "\n");
    assert!(matches!(rt.state.overlay, Overlay::None), "Enter closes it");
    typ(&mut rt, ":h\n");
    assert!(picking(&rt.state, PickKind::Keys), ":h");
    typ(&mut rt, "\x1b");
    typ(&mut rt, "i");
    f1(&mut rt);
    assert!(picking(&rt.state, PickKind::Keys), "F1 works in Insert mode");
    typ(&mut rt, "\x1b\x1b");

    let mut a = super::app("classic");
    a.tree.index = crate::app::tree::TreeIndex::build(&["a.txt".to_string(), "b.txt".to_string()]);
    let mut rt = run(a);
    f1(&mut rt);
    let all = key_rows(&rt.state);
    assert!(all.iter().any(|r| r.starts_with("Ctrl D ")) && !all.iter().any(|r| r.starts_with("SPC f ")), "the classic keys");
    assert!(all.iter().any(|r| r.starts_with("Show every key ")), "the palette entries are in the classic list");
    typ(&mut rt, "\x1b");
    press(&mut rt, Key::Text("E".into()), Mods { ctrl: true, cmd: true, shift: true, ..Default::default() });
    assert!(rt.state.tree.show && rt.state.tree.focus, "Ctrl ⇧ E focuses the tree");
    typ(&mut rt, "?");
    assert!(picking(&rt.state, PickKind::Keys), "? in the tree shows every key");
}

/// The wheel over the tree scrolls it a row per 22 points, adds up a trackpad's small steps
/// and leaves the keyboard row alone. Keys bring the keyboard row back on screen.
#[test]
fn the_wheel_scrolls_the_tree_and_keys_bring_the_row_back() {
    let mut a = super::app("neovim");
    let files: Vec<String> = (0..200).map(|i| format!("f{i:03}.txt")).collect();
    a.tree.index = crate::app::tree::TreeIndex::build(&files);
    let mut rt = run(a);
    let top = |rt: &Runtime<App>| rt.state.tree_top(&rt.state.tree.index.rows(&rt.state.tree.open));
    let sel = |rt: &Runtime<App>| rt.state.tree.sel.clone().unwrap_or_default();
    let visible = rt.state.tree_visible();
    let max = 200 - visible;
    let wheel = |rt: &mut Runtime<App>, dy: f32| {
        let mut ev = rune::WheelEvent::default();
        (ev.x, ev.y, ev.dy) = (60.0, 300.0, dy);
        rt.wheel_event(ev);
        rt.frame(1.0 / 60.0);
    };
    wheel(&mut rt, -220.0);
    assert_eq!(top(&rt), 10, "ten rows down");
    assert_eq!(rt.state.tree.sel, None, "the wheel leaves the keyboard row");
    for _ in 0..11 {
        wheel(&mut rt, -2.0);
    }
    assert_eq!(top(&rt), 11, "eleven 2-point trackpad steps make one row");
    wheel(&mut rt, 22.0 * 500.0);
    assert_eq!(top(&rt), 0, "it stops at the first row");
    wheel(&mut rt, -22.0 * 500.0);
    assert_eq!(top(&rt), max, "it stops with the last row at the bottom");

    leader(&mut rt, "e");
    assert!(rt.state.tree.focus);
    typ(&mut rt, "j");
    assert_eq!(sel(&rt), "f001.txt");
    assert_eq!(top(&rt), 1, "j scrolls back to the keyboard row");
    ctrl(&mut rt, 'd');
    assert_eq!(sel(&rt), format!("f{:03}.txt", 1 + visible / 2), "Ctrl d moves half a screen");
    ctrl(&mut rt, 'u');
    assert_eq!(sel(&rt), "f001.txt", "Ctrl u moves back");
    press(&mut rt, Key::PageDown, Mods::default());
    assert_eq!(sel(&rt), format!("f{:03}.txt", visible), "PageDown moves a screen");
    typ(&mut rt, "G");
    assert_eq!((sel(&rt), top(&rt)), ("f199.txt".to_string(), max), "G goes to the last row and shows it");
    wheel(&mut rt, 22.0 * 5.0);
    assert_eq!(top(&rt), max - 5, "the wheel scrolls while the tree has the focus");
    typ(&mut rt, "k");
    assert_eq!((sel(&rt), top(&rt)), ("f198.txt".to_string(), 198 + 1 - visible), "k shows the row at the bottom edge");

    rt.update(|s: &mut App, _| s.cfg.mouse.scroll = false);
    let before = top(&rt);
    wheel(&mut rt, -22.0 * 5.0);
    assert_eq!(top(&rt), before, "mouse.scroll off leaves the tree still");
}
