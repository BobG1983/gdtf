//! Fail if any tracked Rust file hands the frame clock back to Bevy.

use crate::no_restored_automatic_time::{
    message::restored_clock_message,
    scan::offending_lines,
    tree::{repo_root, rust_files},
};

#[test]
fn no_tracked_rust_file_hands_the_frame_clock_back_to_bevy() {
    let root = repo_root();
    assert!(
        root.join("crates").is_dir(),
        "no crates directory under {}. This guard is reading the wrong directory and \
         cannot see any source files",
        root.display()
    );

    let mut found: Vec<String> = rust_files(&root)
        .iter()
        .flat_map(|path| offending_lines(&root, path))
        .collect();
    found.sort();

    assert!(found.is_empty(), "{}", restored_clock_message(&found));
}
