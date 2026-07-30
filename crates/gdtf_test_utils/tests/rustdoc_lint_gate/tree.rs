//! Workspace-member enumeration — `git ls-files` over the two member
//! directories with a std fs-walk fallback, mirroring the sibling guards'
//! recipe.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// The two directories the workspace's `members` globs cover.
///
/// `check.rs` asserts the workspace still declares exactly these two globs, so
/// a member added under a third directory fails the guard rather than escaping
/// this walk.
pub(crate) const MEMBER_DIRS: [&str; 2] = ["crates", "bins"];

/// The repo root — `GDTF_RUSTDOC_GATE_ROOT` override, else
/// `CARGO_MANIFEST_DIR/../..`.
pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_RUSTDOC_GATE_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// Tracked `Cargo.toml` paths one level under `dir` via `git ls-files`; `None`
/// if git is unavailable or errors (not a repo).
fn git_tracked(root: &Path, dir: &str) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", &format!("{dir}/*/Cargo.toml")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

/// fs-walk fallback: `<dir>/<member>/Cargo.toml` for every subdirectory of
/// `dir` that has one, pushed as a forward-slash path relative to `root`.
fn walk_manifests(root: &Path, dir: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| format!("{dir}/{}/Cargo.toml", entry.file_name().to_string_lossy()))
        .filter(|path| root.join(path).is_file())
        .collect()
}

/// Every workspace member's `Cargo.toml`, sorted — git enumeration first,
/// fs-walk fallback.
///
/// Only the manifest ONE level under each member directory is a member: the
/// globs are `crates/*` and `bins/*`, so a nested manifest deeper in a crate's
/// tree is not a workspace member and is deliberately not walked.
pub(crate) fn member_manifests(root: &Path) -> Vec<String> {
    let mut manifests: Vec<String> = MEMBER_DIRS
        .iter()
        .flat_map(|dir| git_tracked(root, dir).unwrap_or_else(|| walk_manifests(root, dir)))
        .filter(|path| path.matches('/').count() == 2)
        .collect();
    manifests.sort();
    manifests.dedup();
    manifests
}
