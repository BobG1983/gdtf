use std::path::PathBuf;

use crate::{
    capture::{
        ShotDir, ShotStem,
        dir::{DEFAULT_STEM, ShotSequence, next_capture_path},
    },
    path::CapturePath,
};

fn test_dir() -> ShotDir {
    ShotDir::new(PathBuf::from("target/test_shots"))
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
