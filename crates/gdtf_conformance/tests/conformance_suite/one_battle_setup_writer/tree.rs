//! Repo layout helpers for the battle-setup-writer guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// The directory the guard reads, relative to the repo root.
pub(crate) const AREA: &str = "crates/gdtf_game/src";

// The workspace root, or an empty path when none is found above this crate.
pub(crate) fn repo_root() -> PathBuf {
    let root = cobalt_ron_assets::workspace_root().unwrap_or_default();
    root.canonicalize().unwrap_or(root)
}

// Every `*.rs` file under `crates/gdtf_game/src`, as repo-relative paths, sorted.
pub(crate) fn rust_files(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).map_or_else(
        || read_area(root),
        |tracked| tracked.into_iter().filter(|path| is_rust(path)).collect(),
    );
    files.sort();
    files
}

// Tracked files under the area. None when git cannot answer.
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", AREA])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn is_rust(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext == "rs")
}

// Fallback for when git is unavailable: walk the area directory.
fn read_area(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    read_dir_into(&root.join(AREA), AREA, &mut found);
    found
}

fn read_dir_into(dir: &Path, prefix: &str, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}/{name}");
        if entry.path().is_dir() {
            read_dir_into(&entry.path(), &path, found);
        } else if is_rust(&path) {
            found.push(path);
        }
    }
}
