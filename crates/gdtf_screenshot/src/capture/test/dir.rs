use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::{
    capture::{
        ShotDir, ShotDirName, ShotStem,
        dir::{DEFAULT_STEM, ShotSequence, next_capture_path, workspace_root_from},
    },
    path::CapturePath,
};

fn test_dir() -> ShotDir {
    ShotDir::new(PathBuf::from("target/test_shots"))
}

fn assert_no_parent_hop(path: &Path) {
    assert!(
        !path.components().any(|part| part.as_os_str() == ".."),
        "a shot path must not carry a `..` segment: {}",
        path.display(),
    );
}

#[test]
fn the_default_shot_directory_is_absolute_and_under_the_workspace_target() {
    let dir = ShotDir::default();
    assert!(
        dir.is_absolute(),
        "a relative default writes a stray `target/` under whatever directory the process runs \
         from; got {}",
        dir.display(),
    );
    assert_no_parent_hop(dir.as_path());
    let target = dir.parent();
    assert_eq!(
        dir.file_name(),
        Some(OsStr::new("qa_screenshots")),
        "the default keeps its own name as the last component; got {}",
        dir.display(),
    );
    assert_eq!(
        target.and_then(Path::file_name),
        Some(OsStr::new("target")),
        "the default must sit directly under a `target/`, which the root `.gitignore` covers; \
         got {}",
        dir.display(),
    );
    assert!(
        target
            .and_then(Path::parent)
            .is_some_and(|root| root.join("Cargo.lock").is_file()),
        "that `target/` must hang off the workspace root, the only directory holding a \
         `Cargo.lock`; got {}",
        dir.display(),
    );
}

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
fn a_named_shot_directory_sits_beside_the_default_under_one_target() {
    let named = ShotDir::under_workspace_target(&ShotDirName::new("qa_screenshot_gpu_1"));
    let default = ShotDir::default();
    assert!(
        named.is_absolute(),
        "a named shot directory must be absolute, got {}",
        named.display(),
    );
    assert_no_parent_hop(named.as_path());
    assert_eq!(
        named.parent(),
        default.parent(),
        "every named shot directory shares the workspace `target/` with the default",
    );
}

#[test]
fn a_capture_under_the_default_directory_is_written_outside_this_crate() {
    let dir = ShotDir::default();
    let path = next_capture_path(
        &dir,
        Some(&ShotStem::new("shot")),
        &mut ShotSequence::default(),
    );
    assert!(
        path.is_absolute(),
        "a capture path must be absolute, got {}",
        path.display(),
    );
    assert_no_parent_hop(path.as_path());
    assert!(
        !path.starts_with(env!("CARGO_MANIFEST_DIR")),
        "a capture must not land inside this crate's own directory: {}",
        path.display(),
    );
}

#[test]
fn a_traversing_name_is_confined_to_the_shot_directory() {
    let dir = test_dir();
    let mut seq = ShotSequence::default();
    for hostile in ["../../etc/passwd", "/etc/passwd", "nested/dir/shot.png"] {
        let path = next_capture_path(
            &dir,
            Some(&ShotStem::new(hostile)),
            &mut ShotSequence::default(),
        );
        assert_eq!(
            path.parent(),
            Some(PathBuf::from("target/test_shots").as_path()),
            "{hostile:?} escaped the confinement directory: {}",
            path.display(),
        );
        assert!(
            !path.components().any(|part| part.as_os_str() == ".."),
            "{hostile:?} left a `..` segment in {}",
            path.display(),
        );
    }
    let fallback = next_capture_path(&dir, None, &mut seq);
    assert_eq!(
        fallback,
        CapturePath::new(PathBuf::from("target/test_shots").join(format!("{DEFAULT_STEM}_0.png"))),
        "an unnamed capture must fall back to the default stem",
    );
}

#[test]
fn a_hostile_charset_is_filtered_out_of_the_stem() {
    let dir = test_dir();
    let hostile = ShotStem::new("sh ell;$(rm -rf *)&|>'\"`.png");
    let path = next_capture_path(&dir, Some(&hostile), &mut ShotSequence::default());
    let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
        unreachable!("a confined path always has a file name");
    };
    assert_eq!(file, "shellrm-rf_0.png", "filtered file name: {file}");
}

#[test]
fn a_repeated_name_still_yields_a_unique_path() {
    let dir = test_dir();
    let mut seq = ShotSequence::default();
    let stem = ShotStem::new("shell");
    let first = next_capture_path(&dir, Some(&stem), &mut seq);
    let second = next_capture_path(&dir, Some(&stem), &mut seq);
    assert_ne!(first, second, "a repeated name must not reuse a path");
}
