//! The leader menu, the command palette and the fixes from the key audit, run against the real
//! app with its real workers: git, the file watcher, ripgrep and clangd.

mod common;

use std::time::{Duration, Instant};

use common::{Flow, SRC, flow, flow_files, keymap_config, rust_flow, scratch, write};
use editor::buffer::Pos;
use editor::editor::{Keymap, Mode, VisualKind};
use nsvim_native::app::{App, Overlay, PickKind};

fn picking(a: &App, kind: PickKind) -> bool {
    matches!(&a.overlay, Overlay::Pick(p) if p.kind == kind)
}

fn said(a: &App, part: &str) -> bool {
    a.ed.msg.as_ref().is_some_and(|m| m.text.contains(part))
}

fn active_ends_with(a: &App, name: &str) -> bool {
    a.ed.has_doc() && a.ed.buf().path.as_deref().is_some_and(|p| p.ends_with(name))
}

/// A C file in git: committed, then changed on lines 4 and 5 and on line 15. `main` calls
/// `twice`, and `broken` holds an error for the language server to report.
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
    let dir = scratch();
    let root = dir.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").args(args).current_dir(&root).output().expect("git");
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "test"]);
    git(&["config", "commit.gpgsign", "false"]);
    write(&root, "main.c", LEADER_C);
    write(&root, "notes.txt", "a needle in the notes\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let changed = LEADER_C
        .replace("int base = 1;", "int base = 2;")
        .replace("return n * 2 * base;", "return n * 2 * base * 1;")
        .replace("return total;", "return total + 0;");
    let main_c = write(&root, "main.c", &changed);
    let mut f = flow(&main_c, None);
    let lsp = std::process::Command::new("clangd").arg("--version").output().is_ok();

    // Top level.
    f.keys(" f");
    f.settle("SPC f opens the file finder", |a| picking(a, PickKind::Files));
    f.keys("\x1b");
    f.keys(" b");
    f.settle("SPC b lists buffers", |a| picking(a, PickKind::Buffers));
    f.keys("\x1b");
    f.keys(" /");
    f.settle("SPC / opens project search", |a| picking(a, PickKind::Grep));
    f.keys("needle");
    f.settle("SPC / finds the needle", |a| !a.grep.hits.is_empty());
    f.keys("\x1b");
    f.keys(" e");
    f.settle("SPC e focuses the tree", |a| a.tree.show && a.tree.focus);
    f.keys("\x1b");
    f.settle("Esc leaves the tree", |a| !a.tree.focus);
    f.keys(" ,");
    f.settle("SPC , opens settings", |a| a.on_settings);
    f.update(|a, _| a.close_settings());
    f.keys(" ?");
    f.settle("SPC ? shows every key", |a| picking(a, PickKind::Keys));
    f.keys("\x1b");
    f.keys(" p");
    f.settle("SPC p opens the command palette", |a| picking(a, PickKind::Commands));
    f.keys("\x1b");

    // +git: the two changed blocks, in order and around.
    f.settle("git signs", |a| a.meta.get(&a.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.signs.is_empty()));
    // From the line just above a two-line change: to its first line, then past its second.
    f.update(|a, _| a.ed.set_cursor(Pos::new(2, 0)));
    f.keys(" gn");
    f.settle("SPC g n goes to the first change", |a| a.ed.cursor().line == 3);
    f.keys(" gn");
    f.settle("SPC g n goes to the next change, not the next line", |a| a.ed.cursor().line == 14);
    f.keys(" gn");
    f.settle("SPC g n wraps to the first change", |a| a.ed.cursor().line == 3);
    f.keys(" gp");
    f.settle("SPC g p wraps to the last change", |a| a.ed.cursor().line == 14);
    f.keys(" gs");
    f.settle("SPC g s focuses the tree", |a| a.tree.focus);
    f.keys("\x1b");

    // +lsp, on `twice` where main calls it.
    if lsp {
        f.update(|a, _| a.ed.set_cursor(Pos::new(12, 17)));
        f.settle("clangd reports the error", |a| a.meta.get(&a.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.diags.is_empty()));
        f.keys(" d");
        f.settle("SPC d lists the error clangd reports", |a| {
            picking(a, PickKind::Locations) && a.locations.iter().any(|l| l.text.contains("missing_name"))
        });
        f.keys("\x1b");
        f.keys(" lh");
        f.settle("SPC l h shows hover", |a| a.hover.as_deref().is_some_and(|h| h.contains("twice")));
        f.keys("\x1b");
        f.keys(" lr");
        f.settle("SPC l r lists references", |a| picking(a, PickKind::Locations) && a.locations.len() >= 2);
        f.keys("\x1b");
        f.update(|a, _| a.ed.set_cursor(Pos::new(12, 17)));
        f.keys(" la");
        f.settle("SPC l a answers with code actions", |a| picking(a, PickKind::Actions) || said(a, "no code actions"));
        if f.check(|a| matches!(a.overlay, Overlay::Pick(_))) {
            f.keys("\x1b");
        }
        f.update(|a, _| a.ed.set_cursor(Pos::new(12, 17)));
        f.keys(" ld");
        f.settle("SPC l d goes to the definition", |a| a.ed.cursor().line == 2);
    } else {
        eprintln!("clangd is not installed: the SPC l and SPC d checks did not run");
    }
    f.update(|a, _| a.ed.set_cursor(Pos::new(2, 11)));
    f.keys(" ln");
    f.settle("SPC l n opens :Rename", |a| a.ed.cmd.as_ref().is_some_and(|c| c.text == "Rename twice"));
    f.keys("\x1b");

    // SPC w writes. Then an outside change is marked, listed and cleared.
    f.keys("ggx");
    f.keys(" w");
    f.settle("SPC w writes the file", |a| !a.ed.buf().is_dirty());
    std::thread::sleep(Duration::from_millis(400));
    std::fs::write(root.join("main.c"), format!("{changed}/* From outside. */\n")).unwrap();
    f.settle("the outside change is marked", |a| !a.outside.is_empty());
    f.keys(" al");
    f.settle("SPC a l lists outside changes", |a| picking(a, PickKind::Locations));
    f.keys("\x1b");
    f.keys(" ac");
    f.settle("SPC a c clears the marks", |a| a.outside.is_empty() && said(a, "cleared"));

    // +agent: notes and proposals.
    f.keys(" ap");
    f.settle("SPC a p with none says so", |a| said(a, "no proposals"));
    let r = f.ask("reveal", serde_json::json!({"path": "main.c", "line": 3, "end_line": 5, "note": "look"}));
    assert!(r.ok, "{:?}", r.error);
    f.settle("the note shows", |a| a.note.is_some());
    f.keys(" an");
    f.settle("SPC a n clears the note", |a| a.note.is_none());
    let notes = root.join("notes.txt");
    f.on_engine(move |s| s.open(notes));
    f.settle("open notes.txt", |a| active_ends_with(a, "notes.txt"));
    let r = f.ask("propose", serde_json::json!({"path": "main.c", "text": "int main(void) { return 0; }\n", "label": "smaller"}));
    assert!(r.ok, "{:?}", r.error);
    f.keys(" ap");
    f.settle("SPC a p shows the proposal", |a| active_ends_with(a, "main.c") && a.proposal_here().is_some());
    f.keys("n");

    // SPC o lists the projects to open.
    f.keys(" o");
    f.settle("SPC o opens the project list", |a| picking(a, PickKind::Projects));
    f.keys("\x1b");

    // SPC q closes the buffer.
    assert!(f.check(|a| a.ed.docs.len() == 2), "two buffers open");
    f.keys(" q");
    f.settle("SPC q closes the buffer", |a| a.ed.docs.len() == 1);

    // Every other key of the menu runs a palette command, which `every_palette_command_does_what_it_says`
    // runs, and `palette_tests` checks that each key finds its command.
}

/// The whole text of the active buffer.
fn text(a: &App) -> String {
    a.ed.buf().slice(0, a.ed.buf().len_chars())
}

/// Runs palette command `id`, waits until `done` holds and presses Esc.
fn run_check(
    f: &mut Flow,
    tested: &mut Vec<&'static str>,
    id: &'static str,
    what: &str,
    done: impl Fn(&App) -> bool + Send + Sync + Clone + 'static,
) {
    f.update(move |a, t| a.run_command(id, t));
    // A result the command sets at once counts before later events can replace it.
    if !f.check(done.clone()) {
        f.settle(what, done);
    }
    tested.push(id);
    f.keys("\x1b");
}

/// Every entry of the command palette, run in the real app with its real workers: each one
/// must do what its label says, or say why it cannot.
#[test]
fn every_palette_command_does_what_it_says() {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut f = flow_files(&[&a, &b], None);
    f.settle("both files open", |a| a.ed.docs.len() == 2 && active_ends_with(a, "b.txt"));
    let mut tested = Vec::new();
    let t = &mut tested;
    run_check(&mut f, t, "find", "find opens the finder", |a| picking(a, PickKind::Files));
    run_check(&mut f, t, "grep", "grep opens project search", |a| picking(a, PickKind::Grep));
    run_check(&mut f, t, "project", "project lists the projects", |a| picking(a, PickKind::Projects));
    f.keys("x");
    run_check(&mut f, t, "save", "save writes the file", |a| !a.ed.buf().is_dirty());
    f.keys("x");
    run_check(&mut f, t, "save-all", "save all writes every file", |a| a.ed.docs.iter().all(|d| !d.buf.is_dirty()));
    let shown = f.read(|a| a.tree.show);
    run_check(&mut f, t, "tree", "tree toggles the file tree", move |a| a.tree.show != shown);
    run_check(&mut f, t, "settings", "settings opens settings", |a| a.on_settings);
    // Close Settings, so the keys go to the files again.
    f.update(|a, _| a.close_settings());
    run_check(&mut f, t, "next", "next goes to the next buffer", |a| active_ends_with(a, "a.txt"));
    run_check(&mut f, t, "prev", "prev goes to the previous buffer", |a| active_ends_with(a, "b.txt"));
    run_check(&mut f, t, "keymap=classic", "the classic keymap", |a| a.ed.keymap == Keymap::Classic);
    run_check(&mut f, t, "keymap=neovim", "the Neovim keymap", |a| a.ed.keymap == Keymap::Vim);
    for (id, mode) in [("theme=light", "light"), ("theme=system", "system"), ("theme=dark", "dark")] {
        run_check(&mut f, t, id, id, move |a| a.cfg.theme.mode == mode);
    }
    let wrap = f.read(|a| a.cfg.editor.wrap);
    run_check(&mut f, t, "wrap", "wrap toggles soft wrap", move |a| a.cfg.editor.wrap != wrap);
    run_check(&mut f, t, "keys", "keys shows every key", |a| picking(a, PickKind::Keys));
    run_check(&mut f, t, "outside", "outside says when there is nothing", |a| said(a, "no changes from outside"));
    run_check(&mut f, t, "clear-outside", "clear-outside confirms", |a| said(a, "cleared"));
    run_check(&mut f, t, "proposal", "proposal says when there is none", |a| said(a, "no proposals"));
    run_check(&mut f, t, "clear-note", "clear-note clears the note", |a| a.note.is_none());
    run_check(&mut f, t, "buffers", "buffers lists the buffers", |a| picking(a, PickKind::Buffers));
    f.update(|a, _| a.ed.msg = None);
    run_check(&mut f, t, "diagnostics", "diagnostics says when there are none", |a| said(a, "no diagnostics"));
    f.update(|a, _| a.ed.msg = None);
    run_check(&mut f, t, "git-next", "git-next says when there is no change", |a| said(a, "no changes against HEAD"));
    f.update(|a, _| a.ed.msg = None);
    run_check(&mut f, t, "git-prev", "git-prev says when there is no change", |a| said(a, "no changes against HEAD"));
    run_check(&mut f, t, "git-status", "git-status focuses the tree", |a| a.tree.show && a.tree.focus);
    // Hover needs a language server: `every_leader_menu_key_does_what_it_says` checks it with clangd.
    t.push("hover");
    // The Edit and Go menus, which Linux and Windows do not show.
    let before = f.read(text);
    f.keys("x");
    let undone = before.clone();
    run_check(&mut f, t, "undo", "undo takes the edit back", move |a| text(a) == undone);
    run_check(&mut f, t, "redo", "redo makes the edit again", move |a| text(a) != before);
    run_check(&mut f, t, "all", "select all selects every line", |a| a.ed.mode == Mode::Visual(VisualKind::Line));
    f.keys("V");
    run_check(&mut f, t, "copy", "copy yanks the selected line", |a| {
        a.ed.mode == Mode::Normal && a.ed.regs.get(None).is_some_and(|r| r.linewise && !r.text.is_empty())
    });
    let full = f.read(text);
    f.keys("V");
    let cut = full.clone();
    run_check(&mut f, t, "cut", "cut deletes the selected line", move |a| text(a).len() < cut.len());
    // Paste reads the clipboard in a task that only key events drive, so this one goes the way
    // a person goes: `Ctrl ⇧ P` in the Neovim keymap, the name, Enter.
    f.press("ctrl+shift+p");
    f.settle("Ctrl Shift P opens the palette in the Neovim keymap", |a| picking(a, PickKind::Commands));
    f.keys("paste");
    f.settle("the query picks Paste", |a| {
        matches!(&a.overlay, Overlay::Pick(p) if p.matches.get(p.sel).is_some_and(|m| nsvim_native::app::palette::COMMANDS[m.index].id == "paste"))
    });
    f.press("enter");
    f.settle("paste puts it back", move |a| text(a).len() >= full.len());
    t.push("paste");
    for id in ["definition", "references", "actions"] {
        f.update(|a, _| a.ed.msg = None);
        run_check(&mut f, t, id, id, |a| said(a, "no language server"));
    }
    run_check(&mut f, t, "rename", "rename opens :Rename", |a| a.ed.cmd.as_ref().is_some_and(|c| c.text.starts_with("Rename ")));
    // A test cannot pick a folder in the system's dialog. `palette_tests` checks that it asks.
    t.push("folder");
    f.update(|a, t| a.run_command("save", t));
    f.settle("the edits are saved", |a| !a.ed.buf().is_dirty());
    f.keys("x");
    run_check(&mut f, t, "quit", "quit refuses to drop unsaved edits", |a| {
        a.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("E37"))
    });
    f.keys("u");
    run_check(&mut f, t, "close", "close closes the buffer", |a| a.ed.docs.len() == 1);

    // `:set theme=` takes a theme id too.
    f.keys(":set theme=gruvbox\n");
    f.settle(":set theme=gruvbox", |a| a.cfg.theme.mode == "dark" && a.cfg.theme.dark == "gruvbox");
    f.keys(":set theme=flexoki-light\n");
    f.settle(":set theme=flexoki-light", |a| a.cfg.theme.mode == "light" && a.cfg.theme.light == "flexoki-light");
    f.keys(":set theme=nope\n");
    f.settle(":set theme=nope is an error", |a| a.ed.msg.as_ref().is_some_and(|m| m.error));

    // The commands that close a group of tabs, from the tab that shows.
    let names = |a: &App| a.ed.docs.iter().map(|d| d.buf.name()).collect::<Vec<_>>().join(" ");
    let more: Vec<_> = ["c.txt", "d.txt", "e.txt"].iter().map(|n| write(dir.path(), n, "more\n")).collect();
    let open = |f: &mut Flow, p: &std::path::PathBuf| {
        let q = p.clone();
        f.update(move |a, _| a.open_path(q, None));
        f.wait_open(p);
    };
    f.update(|a, t| a.run_command("save-all", t));
    f.settle("every file is saved", |a| a.ed.docs.iter().all(|d| !d.buf.is_dirty()));
    for p in &more {
        open(&mut f, p);
    }
    f.update(|a, _| a.show_tab(a.ed.docs.len() - 2));
    f.settle("d.txt shows", |a| active_ends_with(a, "d.txt"));
    run_check(&mut f, t, "close-right", "close-right closes e.txt", move |a| !names(a).contains("e.txt") && names(a).contains("d.txt"));
    run_check(&mut f, t, "close-left", "close-left keeps only d.txt", move |a| names(a) == "d.txt");
    open(&mut f, &more[0]);
    open(&mut f, &more[2]);
    f.keys("x");
    run_check(&mut f, t, "close-others", "close-others keeps only e.txt", move |a| names(a) == "e.txt");
    open(&mut f, &more[0]);
    f.update(|a, _| a.ed.msg = None);
    run_check(&mut f, t, "close-clean", "close-clean keeps the modified e.txt", move |a| {
        names(a) == "e.txt" && a.ed.msg.as_ref().is_none_or(|m| !m.error)
    });
    open(&mut f, &more[0]);
    f.update(|a, _| a.show_tab(0));
    f.keys("u");
    f.settle("e.txt is clean again", |a| !a.ed.buf().is_dirty());
    run_check(&mut f, t, "close-all", "close-all closes every tab", |a| a.ed.docs.is_empty());

    // An entry added to the palette must be added here too.
    for c in nsvim_native::app::palette::COMMANDS {
        assert!(tested.contains(&c.id), "\"{}\" ({}) shows in the palette but this test does not run it", c.label, c.id);
    }
}

#[test]
fn wqa_writes_every_buffer_before_it_quits() {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut f = flow_files(&[&a, &b], None);
    f.settle("both files open", |a| a.ed.docs.len() == 2);
    for (i, name) in ["a.txt", "b.txt"].into_iter().enumerate() {
        f.update(move |a, _| a.ed.active = i);
        f.settle("the buffer is active", move |a| active_ends_with(a, name));
        f.keys("x");
    }
    f.keys(":wqa\n");
    assert!(!f.msg().is_some_and(|(_, error)| error), "{:?}", f.msg());
    // The last write quits, and a quit saves the session to the isolated state folder.
    for (file, want) in [(&a, "lpha\n"), (&b, "eta\n")] {
        let start = Instant::now();
        while std::fs::read_to_string(file).unwrap() != want {
            assert!(start.elapsed().as_secs() < 10, "{} was not written", file.display());
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn wqa_waits_for_both_writes_before_it_quits() {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut f = flow_files(&[&a, &b], None);
    f.settle("both files open", |a| a.ed.docs.len() == 2);
    // Both buffers dirty, then `:wqa` in one update, before a write can come back.
    let quit_after = f.update(|a, t| {
        for i in 0..2 {
            a.ed.active = i;
            editor::vim::handle_key(&mut a.ed, editor::editor::KeyInput::ch('x'));
        }
        let actions = editor::vim::run_ex(&mut a.ed, "wqa");
        a.run_actions(actions, t);
        a.quit_after_saves
    });
    assert_eq!(quit_after, Some(2), "it waits for both writes before it quits");
}

/// The file watcher reads back the editor's own save. Typing right after `:w` makes the buffer
/// differ from that read-back, which is not a change from outside.
#[test]
fn typing_right_after_a_save_is_not_a_conflict() {
    let dir = scratch();
    let path = write(dir.path(), "a.txt", "alpha\n");
    let mut f = flow(&path, None);
    f.keys("x:w\n");
    f.settle("the save", |a| !a.ed.buf().is_dirty());
    f.keys("x");
    f.ui.wait(Duration::from_millis(1500)).unwrap();
    let p = path.clone();
    assert!(!f.read(move |a| a.meta.get(&p).is_some_and(|m| m.conflict)), "marked as changed on disk");
    assert!(!f.said("W12"), "{:?}", f.msg());
}

fn key_rows(f: &Flow) -> Vec<String> {
    f.read(|a| {
        let Overlay::Pick(p) = &a.overlay else { return Vec::new() };
        p.matches.iter().map(|m| p.items[m.index].clone()).collect()
    })
}

/// `F1`, `SPC ?`, `:Keys`, `:h` and the tree's `?` show every key, and the search keeps the
/// rows with every word. Each keymap lists its own keys.
#[test]
fn every_key_shows_in_a_list_with_search() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.press("f1");
    assert!(f.check(|a| picking(a, PickKind::Keys)), "F1 opens the list");
    let all = key_rows(&f);
    assert!(
        all.len() > 300 && all.iter().any(|r| r.starts_with("SPC f ")) && !all.iter().any(|r| r.starts_with("Ctrl D ")),
        "{}",
        all.len()
    );
    f.keys("tree");
    let found = key_rows(&f);
    assert!(!found.is_empty() && found.len() < 60, "{} rows", found.len());
    assert!(found.iter().all(|r| r.to_lowercase().contains("tree")), "{found:#?}");
    f.press("f1");
    assert!(f.check(|a| matches!(a.overlay, Overlay::None)), "F1 again closes it");

    f.keys(":Keys dd\n");
    assert!(f.check(|a| picking(a, PickKind::Keys)), ":Keys");
    let found = key_rows(&f);
    assert!(!found.is_empty() && found.iter().all(|r| r.contains("dd")), "{found:#?}");
    f.keys("\n");
    assert!(f.check(|a| matches!(a.overlay, Overlay::None)), "Enter closes it");
    f.keys(":h\n");
    assert!(f.check(|a| picking(a, PickKind::Keys)), ":h");
    f.keys("\x1b");
    f.keys("i");
    f.press("f1");
    assert!(f.check(|a| picking(a, PickKind::Keys)), "F1 works in Insert mode");
    f.keys("\x1b\x1b");

    let dir = scratch();
    write(dir.path(), "b.txt", "b\n");
    let file = write(dir.path(), "a.txt", "a\n");
    let mut f = flow(&file, Some(&keymap_config("classic")));
    f.settle("the project index", |a| a.files.len() == 2);
    f.press("f1");
    let all = key_rows(&f);
    assert!(all.iter().any(|r| r.starts_with("Ctrl D ")) && !all.iter().any(|r| r.starts_with("SPC f ")), "the classic keys");
    assert!(all.iter().any(|r| r.starts_with("Show every key… ")), "the palette entries are in the classic list");
    f.keys("\x1b");
    f.press("ctrl+shift+e");
    assert!(f.read(|a| a.tree.show && a.tree.focus), "Ctrl ⇧ E focuses the tree");
    f.keys("?");
    assert!(f.check(|a| picking(a, PickKind::Keys)), "? in the tree shows every key");
}

/// An edit when the app is quiet still runs the work that follows it. Only a poll runs that
/// work, and before the fix nothing polled after a key: the title kept no `[+]`, and the
/// language server, the git signs and the swap file never heard of the edit.
#[test]
fn an_edit_when_the_app_is_quiet_reaches_the_title_the_server_the_signs_and_the_swap() {
    let dir = scratch();
    let file = write(dir.path(), "quiet.rs", "fn main() {}\n");
    let mut f = flow(&file, Some(&keymap_config("neovim")));
    // Past the swap file's 2 s pause, so no wake from the open is left.
    f.ui.idle(Duration::from_millis(2500)).unwrap();
    f.keys("x");
    let p = file.clone();
    f.settle("the edit reaches every follower", move |a| {
        let (m, next) = (&a.meta[&p], a.ed.buf().version() + 1);
        a.title.contains("[+]") && m.lsp_sent == next && m.signs_asked == next && m.swapped == next
    });
}
