//! Repo layout helpers for the no-flat guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn repo_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root.canonicalize().unwrap_or(root)
}

// Every `*.rs` file sitting directly in a `crates/<name>/tests/`,
// `bins/<name>/tests/` or `libs/<name>/tests/` directory, as repo-relative paths, sorted.
pub(crate) fn flat_test_files(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = test_entries(root)
        .into_iter()
        .filter(|p| is_flat_test(p))
        .collect();
    files.sort();
    files
}

// Every `<area>/<name>/tests/<dir>/main.rs`, the root of one integration-test binary, as
// repo-relative paths, sorted.
pub(crate) fn suite_roots(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = test_entries(root)
        .into_iter()
        .filter(|p| is_suite_root(p))
        .collect();
    files.sort();
    files
}

// Paths under `<area>/<name>/tests/` at most two levels deep: tracked files when git can
// answer, so untracked scratch files are ignored, and a filesystem read otherwise.
fn test_entries(root: &Path) -> Vec<String> {
    git_tracked(root).unwrap_or_else(|| read_test_dirs(root))
}

fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "crates", "bins", "libs"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn is_flat_test(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let [area, _crate_dir, "tests", file] = parts.as_slice() else {
        return false;
    };
    matches!(*area, "crates" | "bins" | "libs")
        && Path::new(file).extension().is_some_and(|e| e == "rs")
}

fn is_suite_root(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let [area, _crate_dir, "tests", _dir, "main.rs"] = parts.as_slice() else {
        return false;
    };
    matches!(*area, "crates" | "bins" | "libs")
}

// Fallback for when git is unavailable: read each `<area>/<name>/tests/` directory and one
// level below it.
fn read_test_dirs(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for area in ["crates", "bins", "libs"] {
        let Ok(crates) = fs::read_dir(root.join(area)) else {
            continue;
        };
        for crate_dir in crates.flatten() {
            let name = crate_dir.file_name().to_string_lossy().into_owned();
            let Ok(entries) = fs::read_dir(crate_dir.path().join("tests")) else {
                continue;
            };
            for entry in entries.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                if entry.path().is_file() {
                    found.push(format!("{area}/{name}/tests/{file}"));
                } else if entry.path().join("main.rs").is_file() {
                    found.push(format!("{area}/{name}/tests/{file}/main.rs"));
                }
            }
        }
    }
    found
}
