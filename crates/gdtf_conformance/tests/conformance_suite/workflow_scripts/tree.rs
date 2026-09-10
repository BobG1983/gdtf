//! Repo layout helpers for the workflow-script guards.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Where the workflow scripts live, relative to the repo root.
pub(crate) const WORKFLOW_DIR: &str = ".claude/workflows";

// The workspace root, or an empty path when none is found above this crate.
pub(crate) fn repo_root() -> PathBuf {
    let root = cobalt_ron_assets::workspace_root().unwrap_or_default();
    root.canonicalize().unwrap_or(root)
}

// Every tracked `*.js` file under the workflow directory, as repo-relative paths, sorted.
pub(crate) fn workflow_scripts(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).map_or_else(
        || read_workflow_dir(root),
        |tracked| tracked.into_iter().filter(|path| is_js(path)).collect(),
    );
    files.sort();
    files
}

// Tracked files under the workflow directory. None when git cannot answer.
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", WORKFLOW_DIR])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn is_js(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext == "js")
}

// Fallback for when git is unavailable: read the workflow directory itself.
fn read_workflow_dir(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join(WORKFLOW_DIR)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| format!("{WORKFLOW_DIR}/{}", entry.file_name().to_string_lossy()))
        .filter(|path| is_js(path))
        .collect()
}
