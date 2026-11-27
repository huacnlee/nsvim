//! The leader menu, the command palette and the fixes from the key audit, run against the real
//! app with its real workers: git, the file watcher, ripgrep and clangd.

mod common;

use std::time::{Duration, Instant};

use common::{Flow, SRC, flow, flow_files, keymap_config, rust_flow, scratch, write};
use editor::buffer::Pos;
use editor::editor::{Keymap, Mode, VisualKind};
use nsvim_native::app::{App, Overlay, PickKind, Screen};

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
    f.keys(" gb");
    f.settle("SPC g b shows who changed each line", |a| {
        let m = a.meta.get(&a.ed.buf().path.clone().unwrap());
        a.blame
            && m.is_some_and(|m| {
                m.blame.lines.get(3).is_some_and(|b| b.who == "Not committed yet") && m.blame.lines[0].who.starts_with("test, ")
            })
    });
    f.keys(" gb");
    f.settle("SPC g b hides it again", |a| !a.blame);

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
    run_check(&mut f, t, "setup", "setup runs again over the editor", |a| matches!(a.screen, Screen::Setup(_)));
    // Cancel it, so the keys go to the files again.
    f.update(|a, _| a.cancel_setup());
    f.settle("cancel goes back to the editor", |a| matches!(a.screen, Screen::Editor));
    run_check(&mut f, t, "next", "next goes to the next buffer", |a| active_ends_with(a, "a.txt"));
    run_check(&mut f, t, "prev", "prev goes to the previous buffer", |a| active_ends_with(a, "b.txt"));
    run_check(&mut f, t, "new", "new opens an empty buffer with no name", |a| a.ed.docs.len() == 3 && a.ed.buf().path.is_none());
    // Close it again: a clean buffer closes without a dialog.
    f.update(|a, t| a.close_tab(a.ed.active, t));
    f.settle("the new buffer closes", |a| a.ed.docs.len() == 2 && active_ends_with(a, "b.txt"));
    run_check(&mut f, t, "keymap=classic", "the classic keymap", |a| a.ed.keymap == Keymap::Classic);
    run_check(&mut f, t, "keymap=neovim", "the Neovim keymap", |a| a.ed.keymap == Keymap::Vim);
    for (id, mode) in [("theme=light", "light"), ("theme=system", "system"), ("theme=dark", "dark")] {
        run_check(&mut f, t, id, id, move |a| a.cfg.theme.mode == mode);
    }
    let wrap = f.read(|a| a.cfg.editor.wrap);
    run_check(&mut f, t, "wrap", "wrap toggles soft wrap", move |a| a.cfg.editor.wrap != wrap);
    // Zoom changes the font sizes and writes config.toml at once.
    run_check(&mut f, t, "zoom-in", "zoom-in makes the text larger", |a| a.cfg.font.size == 14.0 && a.cfg.ui.size == 13.5);
    f.settle("config.toml has the new size", |_| {
        std::fs::read_to_string(nsvim_native::config::config_path()).is_ok_and(|t| t.contains("size = 14.0"))
    });
    run_check(&mut f, t, "zoom-out", "zoom-out makes the text smaller", |a| a.cfg.font.size == 13.0 && a.cfg.ui.size == 12.5);
    f.update(|a, t| a.run_command("zoom-in", t));
    run_check(&mut f, t, "zoom-reset", "zoom-reset gives the default sizes", |a| a.cfg.font.size == 13.0 && a.cfg.ui.size == 12.5);
    run_check(&mut f, t, "git-blame", "git blame toggles line blame", |a| a.blame);
    f.update(|a, t| a.toggle_blame(t));
    run_check(&mut f, t, "keys", "keys shows every key", |a| picking(a, PickKind::Keys));
    // A test build is no installed copy, so NSH's updater does not take it. With a release
    // found, the command tries to install it and says why it cannot: no network in the test.
    f.update(|a, _| {
        a.ed.msg = None;
        a.update = Some(nsvim_native::services::update::Release { version: "9.0.0".into(), url: "https://example.com".into() });
    });
    run_check(&mut f, t, "update", "update says why this copy does not install", |a| {
        a.ed.msg.as_ref().is_some_and(|m| m.error && (m.text.contains("9.0.0") || m.text.contains("Save your changes")))
    });
    f.update(|a, _| a.update = None);
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
    run_check(&mut f, t, "line", "line opens Go to Line", |a| picking(a, PickKind::Line));
    run_check(&mut f, t, "symbols", "symbols lists the file's symbols", |a| picking(a, PickKind::Symbols));
    // A jump from b.txt to a.txt: Back opens b.txt again, Forward a.txt.
    f.update(|a, _| {
        editor::vim::remember_jump(&mut a.ed);
        a.ed.active = a.ed.docs.iter().position(|d| d.buf.path.as_ref().is_some_and(|p| p.ends_with("a.txt"))).unwrap();
    });
    run_check(&mut f, t, "back", "back goes to the place before the jump", |a| active_ends_with(a, "b.txt"));
    run_check(&mut f, t, "forward", "forward goes to the jump again", |a| active_ends_with(a, "a.txt"));
    run_check(&mut f, t, "back", "back again, to b.txt for the next checks", |a| active_ends_with(a, "b.txt"));
    // Hover needs a language server: `every_leader_menu_key_does_what_it_says` checks it with clangd.
    t.push("hover");
    run_check(&mut f, t, "find-in-file", "find-in-file opens the find bar", |a| a.find.as_ref().is_some_and(|b| !b.replace));
    run_check(&mut f, t, "replace-in-file", "replace-in-file opens the replace field", |a| a.find.as_ref().is_some_and(|b| b.replace));
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
    // The line commands of the Edit menu, on a buffer of two lines.
    f.update(|a, _| {
        let buf = &mut a.ed.doc_mut().buf;
        let n = buf.len_chars();
        buf.begin(Pos::new(0, 0));
        buf.remove(0, n);
        buf.insert_at(0, "one\ntwo\n");
        buf.commit(Pos::new(0, 0));
        a.ed.set_cursor(Pos::new(0, 0));
    });
    run_check(&mut f, t, "comment", "comment comments the line out", |a| text(a) == "// one\ntwo\n");
    run_check(&mut f, t, "line-down", "line-down moves the line down", |a| text(a) == "two\n// one\n");
    run_check(&mut f, t, "line-up", "line-up moves the line up", |a| text(a) == "// one\ntwo\n");
    run_check(&mut f, t, "indent", "indent indents the line", |a| text(a).starts_with(char::is_whitespace));
    run_check(&mut f, t, "outdent", "outdent takes the indent away", |a| text(a) == "// one\ntwo\n");
    run_check(&mut f, t, "delete-line", "delete-line deletes the line", |a| text(a) == "two\n");
    for id in ["definition", "references", "actions", "format"] {
        f.update(|a, _| a.ed.msg = None);
        run_check(&mut f, t, id, id, |a| said(a, "no language server"));
    }
    f.update(|a, _| a.ed.msg = None);
    run_check(&mut f, t, "format-selection", "format selection asks for a selection", |a| said(a, "select the text"));
    // The palette and the menus ask for the new name in a dialog, not on the command line. The
    // next step answers it as the test's handler does: Cancel.
    f.update(|a, t| a.run_command("rename", t));
    t.push("rename");
    f.keys("\x1b");
    f.ui.wait_until("the rename dialog", Duration::from_secs(5), |s| {
        s.dialogs().iter().any(|d| d.title.as_deref().is_some_and(|t| t.starts_with("Rename")))
    })
    .unwrap();
    f.settle("rename leaves the command line closed", |a| a.ed.cmd.is_none());
    // A test cannot pick a folder in the system's dialog. `palette_tests` checks that it asks.
    t.push("folder");
    f.update(|a, t| a.run_command("save", t));
    f.settle("the edits are saved", |a| !a.ed.buf().is_dirty());
    f.keys("x");
    run_check(&mut f, t, "quit", "quit asks before it drops unsaved edits", |a| a.ed.buf().is_dirty() && a.asking_quit);
    // The next step answers the dialog as the test's handler does: Cancel. The edits and the
    // window stay.
    f.keys("u");
    f.ui.wait_until("the unsaved changes dialog", Duration::from_secs(5), |s| {
        s.dialogs().iter().any(|d| d.title.as_deref().is_some_and(|t| t.starts_with("Save changes to")))
    })
    .unwrap();
    f.settle("cancel keeps the window", |a| !a.asking_quit && a.ed.has_doc());
    run_check(&mut f, t, "close", "close closes the buffer", |a| a.ed.docs.len() == 1);
    // Reopen closed tab takes the closed files back, the last closed first.
    let first = f.read(|a| a.ed.docs[0].buf.path.clone().unwrap());
    f.update(|a, t| a.close_tab(0, t));
    f.settle("the last tab closes", |a| a.ed.docs.is_empty());
    let last = first.clone();
    run_check(&mut f, t, "reopen", "reopen opens the last closed file", move |a| {
        a.ed.docs.len() == 1 && a.ed.buf().path.as_ref() == Some(&last)
    });
    f.update(|a, t| a.run_command("reopen", t));
    f.settle("reopen again opens the file closed before it", move |a| {
        a.ed.docs.len() == 2 && a.ed.buf().path.as_ref().is_some_and(|p| *p != first)
    });

    // `:set theme=` takes a theme id too.
    f.keys(":set theme=gruvbox\n");
    f.settle(":set theme=gruvbox", |a| a.cfg.theme.mode == "dark" && a.cfg.theme.dark == "gruvbox");
    f.keys(":set theme=flexoki-light\n");
    f.settle(":set theme=flexoki-light", |a| a.cfg.theme.mode == "light" && a.cfg.theme.light == "flexoki-light");
    f.keys(":set theme=nope\n");
    f.settle(":set theme=nope is an error", |a| a.ed.msg.as_ref().is_some_and(|m| m.error));

    // The pane commands ask the panes' screen, which `tests/panes.rs` runs.
    for id in ["split-right", "split-down", "pane-next", "pane-prev", "pane-close"] {
        run_check(&mut f, t, id, &format!("{id} asks the panes"), move |a| a.pane_ask.is_some_and(|(_, c)| c.id() == id));
    }

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

/// Waits until a dialog titled `title` shows, and returns its button labels.
fn dialog_buttons(f: &mut Flow, title: &str) -> Vec<String> {
    let t = title.to_string();
    f.ui.wait_until("the dialog", Duration::from_secs(10), move |s| s.dialogs().iter().any(|d| d.title.as_deref() == Some(t.as_str())))
        .unwrap();
    let d = f.ui.pending_dialog().expect("a dialog");
    d.buttons.iter().map(|b| b.text.clone()).collect()
}

fn press_dialog(f: &mut Flow, button: &str) {
    f.ui.answer_dialog(nsh_testing::DialogAnswer::Press(button.into())).unwrap();
}

fn conflicted(a: &App, path: &std::path::Path) -> bool {
    a.meta.get(path).is_some_and(|m| m.conflict)
}

/// `a.txt` edited here, then changed on disk by another program: the editor asks in a dialog.
fn changed_on_disk_and_here() -> (tempfile::TempDir, std::path::PathBuf, Flow) {
    let dir = scratch();
    let path = write(dir.path(), "a.txt", "alpha\n");
    let mut f = flow(&path, None);
    f.ui.answer_dialogs_myself();
    f.keys("x");
    f.settle("the edit", |a| a.ed.buf().is_dirty());
    // The watcher starts with the workers. Write until it sees a change.
    let title = "\"a.txt\" changed on disk";
    for n in 0..20 {
        std::fs::write(&path, format!("from disk {n}\n")).unwrap();
        let ok =
            f.ui.wait_until("the dialog", Duration::from_millis(500), |s| s.dialogs().iter().any(|d| d.title.as_deref() == Some(title)));
        if ok.is_ok() {
            break;
        }
    }
    assert_eq!(dialog_buttons(&mut f, title), ["Reload from disk", "Keep mine"]);
    assert!(f.said("W12"), "the message line still names :e! and :w: {:?}", f.msg());
    (dir, path, f)
}

/// Reload from disk drops the edit and shows the disk copy, with no Vim command.
#[test]
fn reload_from_disk_in_the_changed_on_disk_dialog_drops_the_edit() {
    let (_dir, path, mut f) = changed_on_disk_and_here();
    press_dialog(&mut f, "Reload from disk");
    let p = path.clone();
    f.settle("the disk copy", move |a| a.ed.buf().text().starts_with("from disk") && !a.ed.buf().is_dirty() && !conflicted(a, &p));
    assert_eq!(f.text(), std::fs::read_to_string(&path).unwrap());
}

/// Keep mine leaves the edit, and the next save writes it over the disk copy.
#[test]
fn keep_mine_in_the_changed_on_disk_dialog_keeps_the_edit() {
    let (_dir, path, mut f) = changed_on_disk_and_here();
    press_dialog(&mut f, "Keep mine");
    f.settle("the answer", |a| said(a, "kept your edits"));
    assert_eq!(f.text(), "lpha\n");
    let p = path.clone();
    assert!(f.read(move |a| a.ed.buf().is_dirty() && conflicted(a, &p)), "still edited, still marked changed on disk");
    f.keys(":w\n");
    let p = path.clone();
    f.settle("the save", move |a| !a.ed.buf().is_dirty() && !conflicted(a, &p));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "lpha\n");
}

/// `b.txt` opened with a swap file a crash left: the editor shows the recovered edits and asks.
fn opened_with_recovered_edits() -> (tempfile::TempDir, std::path::PathBuf, Flow) {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "on disk\n");
    common::isolate_home();
    nsvim_native::services::io::write_swap(b.clone(), "recovered\n".into());
    let mut f = flow(&a, None);
    f.ui.answer_dialogs_myself();
    let p = b.clone();
    f.update(move |a, _| a.open_path(p, None));
    let buttons = dialog_buttons(&mut f, "Recover unsaved edits to \"b.txt\"?");
    assert_eq!(buttons, ["Keep recovered edits", "Discard"]);
    assert!(f.said("E325"), "the message line still names :w and :e!: {:?}", f.msg());
    assert_eq!(f.text(), "recovered\n", "the buffer holds the recovered edits while the dialog shows");
    (dir, b, f)
}

/// Keep recovered edits leaves them in the buffer, unsaved, and a save writes them.
#[test]
fn keep_in_the_recovered_edits_dialog_keeps_them() {
    let (_dir, path, mut f) = opened_with_recovered_edits();
    press_dialog(&mut f, "Keep recovered edits");
    f.settle("the answer", |a| said(a, "kept the recovered edits"));
    assert_eq!(f.text(), "recovered\n");
    assert!(f.read(|a| a.ed.buf().is_dirty()));
    f.keys(":w\n");
    f.settle("the save", |a| !a.ed.buf().is_dirty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "recovered\n");
}

/// Discard drops the recovered edits and their swap file, and shows the disk copy.
#[test]
fn discard_in_the_recovered_edits_dialog_drops_them() {
    let (_dir, path, mut f) = opened_with_recovered_edits();
    press_dialog(&mut f, "Discard");
    f.settle("the disk copy", |a| a.ed.buf().text() == "on disk\n" && !a.ed.buf().is_dirty());
    let swap = nsvim_native::services::io::swap_file(&path);
    let start = Instant::now();
    while swap.exists() {
        assert!(start.elapsed() < Duration::from_secs(5), "the swap file stays");
        f.ui.wait(Duration::from_millis(50)).unwrap();
    }
}
