//! Fail unless exactly one file in the game crate writes the battle-setup request.

use crate::one_battle_setup_writer::{
    scan::{MESSAGE, needle, writes_the_request},
    tree::{AREA, repo_root, rust_files},
};

#[test]
fn one_system_in_the_game_crate_writes_the_battle_setup_request() {
    let root = repo_root();
    assert!(
        root.join(AREA).is_dir(),
        "no {AREA} directory under {}. This guard is reading the wrong directory and cannot \
         see any source files",
        root.display()
    );

    let found: Vec<String> = rust_files(&root)
        .into_iter()
        .filter(|path| writes_the_request(&root, path))
        .collect();

    assert_eq!(
        found.len(),
        1,
        "exactly one file under {AREA} may take a {}, and {} do:\n{}\n\nOne route generates \
         the map. A second writer of {MESSAGE} is a second route, and the two drift.",
        needle(),
        found.len(),
        found.join("\n"),
    );
}
