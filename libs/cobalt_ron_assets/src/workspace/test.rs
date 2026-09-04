use super::root::workspace_root_from;

#[test]
fn the_workspace_root_search_finds_the_same_top_from_any_depth() {
    let Ok(shallow) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let Ok(deep) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    for (tree, below) in [(&shallow, "crates/a"), (&deep, "libs/group/nested/a")] {
        let root = tree.path();
        assert!(
            std::fs::write(root.join("Cargo.lock"), "").is_ok(),
            "the test tree needs a lock file at {}",
            root.display(),
        );
        let start = root.join(below);
        assert!(
            std::fs::create_dir_all(&start).is_ok(),
            "the test tree needs a start directory at {}",
            start.display(),
        );
        assert_eq!(
            workspace_root_from(&start).as_deref(),
            Some(root),
            "the search must land on the `Cargo.lock` holder whatever the depth below it",
        );
    }
}

#[test]
fn the_workspace_root_search_finds_a_workspace_manifest_without_a_lock_file() {
    let Ok(tree) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let root = tree.path();
    assert!(
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").is_ok(),
        "the test tree needs a workspace manifest at {}",
        root.display(),
    );
    let start = root.join("crates/a");
    assert!(
        std::fs::create_dir_all(&start).is_ok(),
        "the test tree needs a start directory at {}",
        start.display(),
    );
    assert_eq!(
        workspace_root_from(&start).as_deref(),
        Some(root),
        "with no lock file the search falls back to the `[workspace]` manifest",
    );
}

#[test]
fn the_workspace_root_search_fails_when_neither_marker_is_above_the_start() {
    let Ok(tree) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let start = tree.path().join("crates/a");
    assert!(
        std::fs::create_dir_all(&start).is_ok(),
        "the test tree needs a start directory at {}",
        start.display(),
    );
    assert_eq!(
        workspace_root_from(&start),
        None,
        "no `Cargo.lock` and no `[workspace]` manifest above {} means no root",
        start.display(),
    );
}

#[test]
fn the_assets_root_sits_under_the_workspace_root() {
    let Some(root) = super::root::workspace_root() else {
        unreachable!("this repo has a Cargo.lock above every crate");
    };
    assert_eq!(
        super::root::workspace_assets_root(),
        Some(root.join("assets")),
        "the assets helper only joins `assets` onto the searched root",
    );
}
