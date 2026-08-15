//! The assets root a QA-driven save writes under.

use std::path::PathBuf;

use bevy::prelude::*;

/// Root every QA-driven editor save writes its family folder under.
#[derive(Resource, Clone, Debug, Deref)]
pub struct EditorQaAssetsRoot(PathBuf);

impl EditorQaAssetsRoot {
    /// Point QA-driven saves at this root.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self(root)
    }
}

impl Default for EditorQaAssetsRoot {
    fn default() -> Self {
        Self(workspace_target())
    }
}

// A temp directory when the marker search fails, so a miss never writes inside the repo.
fn workspace_target() -> PathBuf {
    gdtf_assets::workspace_assets_root().unwrap_or_else(std::env::temp_dir)
}
