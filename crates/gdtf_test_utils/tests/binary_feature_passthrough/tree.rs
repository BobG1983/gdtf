//! Binary-manifest enumeration — `git ls-files` over `bins/` with a std
//! fs-walk fallback, mirroring the `module_layout` and `docs_path_truth`
//! guards' recipe.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// The repo root — `GDTF_BIN_PASSTHROUGH_ROOT` override, else
/// `CARGO_MANIFEST_DIR/../..`.
pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_BIN_PASSTHROUGH_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// Tracked files under `bins/` via `git ls-files`; `None` if git is
/// unavailable or errors (not a repo).
fn git_tracked(root: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "bins"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

/// fs-walk fallback: every file under `dir`, skipping `target/` and dot-dirs,
/// pushed as a forward-slash path relative to `root`.
fn walk_files(dir: &Path, root: &Path, acc: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
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

/// Every tracked `bins/<package>/Cargo.toml`, sorted — git enumeration first,
/// fs-walk fallback. Nested manifests (a `Cargo.toml` deeper than one level
/// under `bins/`) are not binary-package roots and are excluded.
pub(crate) fn binary_manifests(root: &Path) -> Vec<String> {
    let mut files = git_tracked(root).unwrap_or_else(|| {
        let mut acc = Vec::new();
        walk_files(&root.join("bins"), root, &mut acc);
        acc
    });
    files.retain(|p| {
        let parts: Vec<&str> = p.split('/').collect();
        matches!(parts.as_slice(), ["bins", _, "Cargo.toml"])
    });
    files.sort();
    files
}
