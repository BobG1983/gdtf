//! Repo layout helpers for the no-flat guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

// Every `*.rs` file sitting directly in a `crates/<name>/tests/` or
// `bins/<name>/tests/` directory, as repo-relative paths, sorted.
pub(crate) fn flat_test_files(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).map_or_else(
        || read_test_dirs(root),
        |tracked| tracked.into_iter().filter(|p| is_flat_test(p)).collect(),
    );
    files.sort();
    files
}

// Tracked files under crates/ and bins/, so untracked scratch files are
// ignored. None when git cannot answer, which sends the caller to the
// filesystem instead.
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "crates", "bins"])
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
    matches!(*area, "crates" | "bins") && Path::new(file).extension().is_some_and(|e| e == "rs")
}

// Fallback for when git is unavailable: read each `<area>/<name>/tests/`
// directory directly.
fn read_test_dirs(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for area in ["crates", "bins"] {
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
                let rs = Path::new(&file).extension().is_some_and(|e| e == "rs");
                if rs && entry.path().is_file() {
                    found.push(format!("{area}/{name}/tests/{file}"));
                }
            }
        }
    }
    found
}
