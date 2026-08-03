//! Workspace-relative path to the shared assets tree.

/// Absolute path to the repo `assets/` folder (via this crate's manifest dir).
pub const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
