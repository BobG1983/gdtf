//! Repo layout helpers for the `libs/` layer guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Directory the promoted, game-free crates live in.
pub(crate) const LIBS_DIR: &str = "libs";

/// Package-name prefix every crate under `libs/` carries.
pub(crate) const COBALT_PREFIX: &str = "cobalt_";

/// Directories that hold gdtf's own crates and binaries.
pub(crate) const GAME_DIRS: [&str; 2] = ["crates", "bins"];

pub(crate) fn repo_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root.canonicalize().unwrap_or(root)
}

// Every `<dir>/<crate>/Cargo.toml` under `dir`, as repo-relative paths, sorted.
pub(crate) fn member_manifests(root: &Path, dir: &str) -> Vec<String> {
    let mut found = git_tracked(root, &format!("{dir}/*/Cargo.toml"))
        .unwrap_or_else(|| walk_manifests(root, dir));
    found.retain(|path| path.matches('/').count() == 2);
    found.sort();
    found.dedup();
    found
}

// Every tracked file under `dir`, as repo-relative paths, sorted. Falls back to a walk when
// git cannot answer.
pub(crate) fn tracked_files(root: &Path, dir: &str) -> Vec<String> {
    let mut found = git_tracked(root, dir).unwrap_or_else(|| walk_files(root, dir));
    found.retain(|path| !path.is_empty());
    found.sort();
    found.dedup();
    found
}

fn walk_files(root: &Path, dir: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.join(dir)];
    while let Some(next) = pending.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found
}

fn git_tracked(root: &Path, pathspec: &str) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", pathspec])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

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
