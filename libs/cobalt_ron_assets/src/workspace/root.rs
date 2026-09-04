//! Marker search for the workspace root, and the assets folder under it.

use std::path::{Path, PathBuf};

/// Workspace root at or above `start`, or `None` when no marker sits above it.
#[must_use]
pub fn workspace_root_from(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
        .or_else(|| start.ancestors().find(|dir| holds_workspace_manifest(dir)))
        .map(Path::to_path_buf)
}

/// Workspace root above this crate, or `None` when the search finds no marker.
#[must_use]
pub fn workspace_root() -> Option<PathBuf> {
    workspace_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// Repo `assets/` folder under the workspace root, or `None` when the search finds no marker.
#[must_use]
pub fn workspace_assets_root() -> Option<PathBuf> {
    workspace_root().map(|root| root.join("assets"))
}

fn holds_workspace_manifest(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|manifest| {
        manifest
            .lines()
            .any(|line| line.trim_start().starts_with("[workspace]"))
    })
}
