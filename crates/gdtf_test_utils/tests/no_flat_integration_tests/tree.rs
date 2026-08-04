//! Repo layout helpers for the no-flat guard.

use std::path::{Path, PathBuf};

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// Crates whose `tests/` trees must not grow flat `*.rs` binaries.
pub(crate) const PACKED_TEST_CRATES: &[&str] = &[
    "crates/gdtf_app",
    "crates/gdtf_assets",
    "crates/gdtf_battle_input",
    "crates/gdtf_battle_presenter",
    "crates/gdtf_battle_sim",
    "crates/gdtf_content_editor",
];
