//! Pure sanitization tests for [`confine_shot_path`] (GTW-740) — driving the REAL confine
//! logic against the REAL default `target/qa_screenshots` directory, so the
//! confinement guarantee is proven where it actually ships.

use std::path::Path;

use gdtf_qa_protocol::ids::ShotName;

use super::super::path::{QaShotDir, ShotSequence, confine_shot_path, next_capture_path};

/// The real default confinement directory the production plugin uses.
fn default_dir() -> QaShotDir {
    QaShotDir::default()
}

/// A confined path must ALWAYS sit under the base directory and never contain a `..`
/// traversal segment.
fn assert_confined(base: &QaShotDir, path: &Path) {
    assert!(
        path.starts_with(&**base),
        "confined path must stay under {}, got {}",
        base.display(),
        path.display(),
    );
    assert!(
        !path.components().any(|c| c.as_os_str() == ".."),
        "confined path must carry no `..` segment, got {}",
        path.display(),
    );
    assert!(
        path.extension().is_some_and(|ext| ext == "png"),
        "confined path must be a .png, got {}",
        path.display(),
    );
}

/// A `../` traversal in the name is stripped — the result stays inside the base directory.
#[test]
fn parent_traversal_is_stripped() {
    let dir = default_dir();
    let name = ShotName::new("../../../etc/passwd".to_owned());
    let confined = confine_shot_path(&dir, Some(&name));
    assert_confined(&dir, &confined);
}

/// An absolute path in the name is confined — only its final component survives, under the
/// base directory.
#[test]
fn absolute_path_is_confined() {
    let dir = default_dir();
    let name = ShotName::new("/tmp/evil.png".to_owned());
    let confined = confine_shot_path(&dir, Some(&name));
    assert_confined(&dir, &confined);
    // The confined file is the final component, sanitized — never the absolute location.
    assert!(
        confined.ends_with("evil.png"),
        "the final component survives sanitization, got {}",
        confined.display(),
    );
}

/// Nested directory segments are dropped — only the final file component is kept.
#[test]
fn nested_directories_are_dropped() {
    let dir = default_dir();
    let name = ShotName::new("some/deep/dir/shot".to_owned());
    let confined = confine_shot_path(&dir, Some(&name));
    assert_confined(&dir, &confined);
    assert!(
        confined.ends_with("shot.png"),
        "only the final component is kept, got {}",
        confined.display(),
    );
}

/// A name with no safe characters (or `None`) falls back to the default stem, still a `.png`
/// under the base directory.
#[test]
fn empty_and_missing_names_fall_back_to_default() {
    let dir = default_dir();
    for name in [
        Some(ShotName::new("///".to_owned())),
        Some(ShotName::new(String::new())),
        None,
    ] {
        let confined = confine_shot_path(&dir, name.as_ref());
        assert_confined(&dir, &confined);
    }
}

/// A well-formed name round-trips to its stem under the base directory (the ordinary case).
#[test]
fn ordinary_name_keeps_its_stem() {
    let dir = default_dir();
    let name = ShotName::new("aim_check".to_owned());
    let confined = confine_shot_path(&dir, Some(&name));
    assert_confined(&dir, &confined);
    assert!(
        confined.ends_with("aim_check.png"),
        "an ordinary stem is preserved, got {}",
        confined.display(),
    );
}

/// The pump's `next_capture_path` still CONFINES a traversal name (uniqueness rides on top of
/// confinement) and carries the sequence suffix `_<n>.png`.
#[test]
fn next_capture_path_confines_a_traversal_name() {
    let dir = default_dir();
    let mut seq = ShotSequence::default();
    let name = ShotName::new("../../../etc/passwd".to_owned());
    let path = next_capture_path(&dir, Some(&name), &mut seq);
    // `assert_confined` already pins the `.png` extension + under-base + no-`..`; here we
    // additionally pin that the sequence suffix rode along.
    assert_confined(&dir, &path);
    let file = path
        .file_name()
        .and_then(|component| component.to_str())
        .unwrap_or_default();
    assert!(
        file.contains('_'),
        "a sequenced path carries a `_<n>` suffix, got {}",
        path.display(),
    );
}

/// Two claims of the SAME name produce DIFFERENT paths — the sequence makes every capture's
/// output unique, so one capture can never clobber (or be mistaken for) another.
#[test]
fn next_capture_path_is_unique_per_claim() {
    let dir = default_dir();
    let mut seq = ShotSequence::default();
    let name = ShotName::new("shot".to_owned());
    let first = next_capture_path(&dir, Some(&name), &mut seq);
    let second = next_capture_path(&dir, Some(&name), &mut seq);
    assert_confined(&dir, &first);
    assert_confined(&dir, &second);
    assert_ne!(
        first.to_string_lossy(),
        second.to_string_lossy(),
        "successive claims of the same name must get distinct paths",
    );
}
