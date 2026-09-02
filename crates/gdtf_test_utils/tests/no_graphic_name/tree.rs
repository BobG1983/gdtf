//! Repo layout helpers for the retired-field guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// The four trees a retired authoring field can hide in without failing a parse.
pub(crate) const SCANNED_ROOTS: &[&str] = &[
    "assets/content",
    "crates/gdtf_app/tests/fixtures",
    "crates/gdtf_content_editor/tests/fixtures",
    "docs",
];

pub(crate) fn repo_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root.canonicalize().unwrap_or(root)
}

/// Every file under the scanned roots, as repo-relative paths, sorted.
pub(crate) fn scanned_files(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| read_roots(root));
    files.sort();
    files
}

// Tracked files under the scanned roots, so untracked scratch files are ignored.
// None when git cannot answer, which sends the caller to the filesystem instead.
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("--")
        .args(SCANNED_ROOTS)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

// Fallback for when git is unavailable: walk each scanned root directly.
fn read_roots(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for scanned in SCANNED_ROOTS {
        walk(root, Path::new(scanned), &mut found);
    }
    found
}

fn walk(root: &Path, relative: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let child = relative.join(entry.file_name());
        if entry.path().is_dir() {
            walk(root, &child, found);
        } else {
            found.push(child.to_string_lossy().replace('\\', "/"));
        }
    }
}
