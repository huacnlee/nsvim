//! The panes: the open files in NSH's `DockArea`. A tab dropped on an edge of a pane splits it,
//! both files show side by side, and a click on a pane gives its file the keys.

mod common;

use std::time::Duration;

use common::{scratch, start_files, write};
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
