//! Workspace-tree enumeration — the tracked production `.rs` files the checker
//! walks. Mirrors the `module_layout` suite: `git ls-files` under `crates/` +
//! `bins/` with a std fs-walk fallback, then the census band classifier is used
//! to keep only PRODUCTION source (the `Logic` and `mod.rs` bands) and drop the
//! test bands. Test scaffolding (loop indices, tempfile paths, assertion
//! counts) is not domain data, so — consistent with the "used as a domain
//! value" qualifier in `.claude/rules/no-bare-types.md` and rule 4's "indices
//! into a collection you own" — the checker does not police it. (This suite's
//! own AC2 fixture proves the checker itself works; the suite files are also
//! hand-kept newtype-clean per the GTW-599 contract.)

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::types::RepoPath;

/// The workspace root — `GDTF_NO_BARE_TYPES_ROOT` override, else
/// `CARGO_MANIFEST_DIR/../..` (the `gdtf_test_utils` crate is two levels down).
pub(crate) fn workspace_root() -> PathBuf {
    std::env::var_os("GDTF_NO_BARE_TYPES_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// Tracked files under `crates/`+`bins/` via `git ls-files`; `None` if git is
/// unavailable or errors.
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
    String::from_utf8(out.stdout)
        .ok()
        .map(|text| text.lines().map(str::to_owned).collect())
}

/// fs-walk fallback: every file under `dir`, skipping `target/` and dot-dirs.
fn walk_files(dir: &Path, root: &Path, acc: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, root, acc);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let parts: Vec<_> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            acc.push(parts.join("/"));
        }
    }
}

/// Whether a repo-relative path is a TEST-band file (integration under
/// `crates/<crate>/tests/`, or an in-src test module/file), using the pinned
/// `module_layout` census precedence. `mod.rs` is production (wiring).
fn is_test_band(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base == "mod.rs" {
        return false;
    }
    let integration = path
        .strip_prefix("crates/")
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(krate, tail)| !krate.is_empty() && tail.starts_with("tests/"));
    integration
        || path.ends_with("/test.rs")
        || path.ends_with("/tests.rs")
        || path.contains("/test/")
        || path.contains("/tests/")
        || base.starts_with("test_")
        || path.contains("test_support")
}

/// All tracked PRODUCTION `.rs` files, sorted — git enumeration first, fs-walk
/// fallback — with the test bands removed.
pub(crate) fn production_rs(root: &Path) -> Vec<RepoPath> {
    let mut files = git_tracked(root).unwrap_or_else(|| {
        let mut acc = Vec::new();
        walk_files(&root.join("crates"), root, &mut acc);
        walk_files(&root.join("bins"), root, &mut acc);
        acc
    });
    files.retain(|p| Path::new(p).extension().is_some_and(|e| e == "rs"));
    files.retain(|p| !is_test_band(p));
    files.sort();
    files.into_iter().map(RepoPath::new).collect()
}
