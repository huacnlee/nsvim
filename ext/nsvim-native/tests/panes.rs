//! The panes: the open files in NSH's `DockArea`. A tab dropped on an edge of a pane splits it,
//! both files show side by side, and a click on a pane gives its file the keys.

mod common;

use std::time::Duration;

use common::{scratch, start_files, typ, type_text, write};
use nsh_engine::{Bounds, Point, Points};
use nsh_testing::{DragOptions, Headless};

const WAIT: Duration = Duration::from_secs(5);

/// The page bounds of the view with test id `id`.
fn bounds(app: &Headless, id: &str) -> Option<Bounds<Points>> {
    app.screen().layout_nodes().into_iter().find(|n| n.test_id.as_deref() == Some(id)).map(|n| n.bounds)
}

/// The page bounds of each pane.
fn panes(app: &Headless) -> Vec<Bounds<Points>> {
    app.screen()
        .layout_nodes()
        .into_iter()
        .filter(|n| n.test_id.as_deref().is_some_and(|t| t.starts_with("pane-")))
        .map(|n| n.bounds)
        .collect()
}

fn file(app: &Headless) -> Option<String> {
    app.screen().accessibility(Some("id:file")).ok().flatten().map(|n| n.name)
}

#[test]
fn a_tab_dropped_on_the_right_edge_splits_the_pane_and_a_click_moves_the_keys() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    // One pane: only the file that shows draws its text.
    assert!(!app.screen().texts().iter().any(|t| t.contains("alpha")), "a hidden tab draws nothing: {:?}", app.screen().texts());
    let area = bounds(&app, "text-area").ok_or("no text area")?;
    let tab = bounds(&app, "tab-1").ok_or("no tab of b.txt")?;
    // From the middle of b.txt's tab to the right edge of the pane: a new pane on the right.
    let from = (tab.origin.x.0 + tab.size.width.0 / 2.0, tab.origin.y.0 + tab.size.height.0 / 2.0);
    let to = (area.origin.x.0 + area.size.width.0 - 12.0, area.origin.y.0 + area.size.height.0 / 2.0);
    app.drag("id:tab-1", Point::new(Points(to.0 - from.0), Points(to.1 - from.1)), DragOptions::default())?;
    app.wait_until("two panes", WAIT, |s| s.texts().iter().any(|t| t.contains("alpha")) && s.texts().iter().any(|t| t.contains("beta")))?;
    let mut shown: Vec<_> = panes(&app).into_iter().filter(|b| b.size.width.0 > 1.0).collect();
    shown.sort_by(|l, r| l.origin.x.0.total_cmp(&r.origin.x.0));
    assert!(shown.len() >= 2, "two panes side by side: {shown:?}");
    // b.txt moved to the new pane and keeps the keys.
    assert_eq!(file(&app).as_deref(), Some("b.txt"));
    let left = shown[0];
    assert!(left.origin.x.0 < area.origin.x.0 + area.size.width.0 / 2.0, "a.txt stays on the left");
    // A click in the left pane gives a.txt the keys.
    app.press_at("text:/alpha/", Point::new(Points(2.0), Points(2.0)))?;
    app.wait_until("a.txt has the keys", WAIT, |s| s.accessibility(Some("id:file")).ok().flatten().is_some_and(|n| n.name == "a.txt"))?;
    // Both files still show.
    assert!(app.screen().texts().iter().any(|t| t.contains("beta")), "b.txt still shows in its pane");
    Ok(())
}

/// The panes that show, left to right, then top to bottom.
fn shown_panes(app: &Headless) -> Vec<Bounds<Points>> {
    let mut v: Vec<_> = panes(app).into_iter().filter(|b| b.size.width.0 > 1.0 && b.size.height.0 > 1.0).collect();
    v.sort_by(|l, r| l.origin.y.0.total_cmp(&r.origin.y.0).then(l.origin.x.0.total_cmp(&r.origin.x.0)));
    v
}

fn has_file(app: &mut Headless, name: &'static str) -> Result<(), String> {
    app.wait_until(name, WAIT, move |s| s.accessibility(Some("id:file")).ok().flatten().is_some_and(|n| n.name == name))
}

#[test]
fn vsplit_and_split_move_the_tab_to_a_new_pane_and_ctrl_w_moves_the_keys() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let c = write(dir.path(), "c.txt", "gamma\n");
    let mut app = start_files(&[&a, &b, &c], None);
    app.wait_for("text:/gamma/")?;
    assert_eq!(shown_panes(&app).len(), 1, "one pane at the start");

    // :vsplit moves c.txt to a new pane on the right. It keeps the keys.
    type_text(&mut app, ":vsplit\n");
    app.wait_until("two panes side by side", WAIT, |s| {
        s.texts().iter().any(|t| t.contains("gamma")) && s.texts().iter().any(|t| t.contains("beta"))
    })?;
    let two = shown_panes(&app);
    assert_eq!(two.len(), 2, "two panes: {two:?}");
    assert!(two[1].origin.x.0 > two[0].origin.x.0 && (two[1].origin.y.0 - two[0].origin.y.0).abs() < 1.0, "side by side: {two:?}");
    has_file(&mut app, "c.txt")?;

    // Ctrl w k gives the keys to the pane on the left, Ctrl w l back to the right.
    typ(&mut app, &["ctrl+w", "k"]);
    has_file(&mut app, "b.txt")?;
    typ(&mut app, &["ctrl+w", "l"]);
    has_file(&mut app, "c.txt")?;

    // Ctrl w h still goes to the tree.
    typ(&mut app, &["ctrl+w", "h"]);
    app.wait_until("the tree has the keys", WAIT, |_| true)?;
    typ(&mut app, &["ctrl+w", "l"]);

    // In the left pane, Ctrl w s moves b.txt to a new pane below: three panes.
    typ(&mut app, &["ctrl+w", "k"]);
    has_file(&mut app, "b.txt")?;
    typ(&mut app, &["ctrl+w", "s"]);
    app.wait_until("three panes", WAIT, |s| s.texts().iter().any(|t| t.contains("alpha")) && s.texts().iter().any(|t| t.contains("beta")))?;
    app.wait_until("three panes show", WAIT, {
        let n = 3;
        move |s| {
            s.layout_nodes()
                .into_iter()
                .filter(|x| {
                    x.test_id.as_deref().is_some_and(|t| t.starts_with("pane-"))
                        && x.bounds.size.width.0 > 1.0
                        && x.bounds.size.height.0 > 1.0
                })
                .count()
                == n
        }
    })?;
    let three = shown_panes(&app);
    let below = three.iter().filter(|p| p.origin.y.0 > three[0].origin.y.0 + 1.0).count();
    assert_eq!(below, 1, "one pane below: {three:?}");
    has_file(&mut app, "b.txt")?;
    Ok(())
}

#[test]
fn close_pane_joins_its_tabs_to_the_pane_next_to_it() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    // One tab in a pane: nothing to split, the command line says why.
    type_text(&mut app, ":vsplit\n");
    app.wait_until("two panes", WAIT, |s| s.texts().iter().any(|t| t.contains("alpha")) && s.texts().iter().any(|t| t.contains("beta")))?;
    typ(&mut app, &["ctrl+w", "v"]);
    app.wait_for("text:/one tab in this pane/")?;
    // Ctrl w c closes the right pane: b.txt joins a.txt's pane, shows and keeps the keys.
    typ(&mut app, &["ctrl+w", "c"]);
    app.wait_until("one pane", WAIT, |s| !s.texts().iter().any(|t| t.contains("alpha")))?;
    assert_eq!(shown_panes(&app).len(), 1, "one pane: {:?}", shown_panes(&app));
    has_file(&mut app, "b.txt")?;
    assert!(bounds(&app, "tab-0").is_some() && bounds(&app, "tab-1").is_some(), "both tabs in the one pane");
    // With one pane, Ctrl w c says so.
    typ(&mut app, &["ctrl+w", "c"]);
    app.wait_for("text:/only one pane/")?;
    Ok(())
}

/// The tabs of the panes' tab bars, as the area lays them out: each one's bounds and whether
/// it holds what `renderTab` draws (a view `tab-<index>`). The row of the tabs is the closest
/// view around both `tab-0` and `tab-1`.
fn dock_tabs(app: &Headless) -> Vec<(Bounds<Points>, bool)> {
    let nodes = app.screen().layout_nodes();
    let up = |id| -> Vec<_> {
        let mut chain = vec![id];
        while let Some(n) = nodes.iter().find(|n| n.id == *chain.last().unwrap()) {
            if n.parent == n.id || chain.contains(&n.parent) {
                break;
            }
            chain.push(n.parent);
        }
        chain
    };
    let by_id = |t: &str| nodes.iter().find(|n| n.test_id.as_deref() == Some(t)).map(|n| n.id);
    let (Some(a), Some(b)) = (by_id("tab-0"), by_id("tab-1")) else { return Vec::new() };
    let (a, b) = (up(a), up(b));
    let Some(&row) = a.iter().find(|id| b.contains(id)) else { return Vec::new() };
    let drawn: Vec<_> = nodes
        .iter()
        .filter(|n| n.test_id.as_deref().is_some_and(|t| t.strip_prefix("tab-").is_some_and(|i| i.parse::<usize>().is_ok())))
        .map(|n| up(n.id))
        .collect();
    nodes
        .iter()
        .filter(|n| n.parent == row)
        .map(|n| (n.bounds, drawn.iter().any(|chain| chain.contains(&n.id))))
        .collect()
}

#[test]
fn many_open_files_each_get_a_drawn_tab() -> Result<(), String> {
    let dir = scratch();
    let first = write(dir.path(), "f00.txt", "zero\n");
    let mut f = common::flow_files(&[&first], None);
    f.ui.wait_for("text:/zero/")?;
    // Opened in one burst, as `nvim *.txt` from a running editor or a folder's files.
    let all: Vec<_> = (1..30).map(|i| write(dir.path(), &format!("f{i:02}.txt"), &format!("file {i}\n"))).collect();
    let paths = all.clone();
    f.update(move |a, _| paths.into_iter().for_each(|p| a.open_path(p, None)));
    f.wait_open(all.last().unwrap());
    f.ui.wait_until("30 tabs", WAIT, |s| s.layout_nodes().iter().any(|n| n.test_id.as_deref() == Some("tab-29")))?;
    let tabs = dock_tabs(&f.ui);
    let blank: Vec<_> = tabs.iter().filter(|(_, drawn)| !drawn).collect();
    assert_eq!(tabs.len(), 30, "one tab for each open file: {tabs:?}");
    assert!(blank.is_empty(), "every tab draws its file: {blank:?}");
    Ok(())
}
