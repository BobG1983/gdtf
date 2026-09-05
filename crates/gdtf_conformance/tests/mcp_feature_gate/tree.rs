//! Tracked-source enumeration for the QA host's gating guard.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// The QA host's own modules, and the files that declare and re-export them.
pub(crate) const MCP_PATHS: [&str; 4] = [
    "crates/gdtf_game/src/dev",
    "crates/gdtf_game/src/lib.rs",
    "crates/gdtf_editor/src/lib.rs",
    "crates/gdtf_editor/src/mcp",
];

// The files that decide whether the QA host is compiled in at all.
pub(crate) const GATE_FILES: [&str; 4] = [
    "crates/gdtf_game/src/dev/mod.rs",
    "crates/gdtf_game/src/dev/plugin.rs",
    "crates/gdtf_game/src/lib.rs",
    "crates/gdtf_editor/src/lib.rs",
];

pub(crate) fn repo_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root.canonicalize().unwrap_or(root)
}

/// Every tracked `.rs` file under [`MCP_PATHS`], as repo-relative paths, sorted.
pub(crate) fn tracked_qa_sources(root: &Path) -> Vec<String> {
    let mut found = git_tracked(root).unwrap_or_else(|| walk(root));
    found.retain(|path| {
        Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
    });
    found.sort();
    found.dedup();
    found
}

pub(crate) fn read(root: &Path, path: &str) -> String {
    let full = root.join(path);
    let Ok(text) = fs::read_to_string(&full) else {
        unreachable!(
            "cannot read {} — a file this guard cannot read is a broken guard, not a pass",
            full.display()
        );
    };
    text
}

fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--"])
        .args(MCP_PATHS)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn walk(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for path in MCP_PATHS {
        collect(root, Path::new(path), &mut found);
    }
    found
}

fn collect(root: &Path, relative: &Path, found: &mut Vec<String>) {
    let full = root.join(relative);
    if full.is_file() {
        found.push(relative.to_string_lossy().into_owned());
        return;
    }
    let Ok(entries) = fs::read_dir(&full) else {
        return;
    };
    for entry in entries.flatten() {
        collect(root, &relative.join(entry.file_name()), found);
    }
}
