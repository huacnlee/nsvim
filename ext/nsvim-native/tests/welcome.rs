//! The welcome page through the real app: a window with no project shows it, and New file
//! takes its place with a buffer.

mod common;

use common::{CONFIG, Flow, flow_with, scratch};

fn welcome(dir: &std::path::Path, recent: std::path::PathBuf) -> Flow {
    let mut f = flow_with(nsvim_native::Launch {
        project: Some(dir.to_path_buf()),
        config: Some(CONFIG.into()),
        files: Vec::new(),
        first_launch: false,
    });
    f.update(move |a, _| a.start_welcome_with(vec![recent]));
    f
}

#[test]
fn the_welcome_page_shows_and_new_file_opens_a_buffer() {
    let dir = scratch();
    let recent = dir.path().join("work/nsh");
    std::fs::create_dir_all(&recent).unwrap();
    let mut f = welcome(dir.path(), recent.clone());
    f.ui.wait_for("id:welcome").unwrap();
    f.ui.wait_for("text:Recent projects").unwrap();
    f.ui.wait_for(&format!("label:{}", nsvim_native::app::project::shown(&recent))).unwrap();
    f.ui.press("label:New file").unwrap();
    f.settle("New file opens a buffer", |a| a.ed.has_doc());
    f.ui.wait_for_gone("id:welcome", std::time::Duration::from_secs(2)).unwrap();
}
