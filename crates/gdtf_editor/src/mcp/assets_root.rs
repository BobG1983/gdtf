//! The assets root the editor's saves write under.

use std::path::PathBuf;

use bevy::prelude::*;

/// Root every editor save writes its family folder under, over the wire or from a button.
/// `MapEditorPlugin` seeds it, so it is there whether or not the QA listener bound.
#[derive(Resource, Clone, Debug, Deref)]
pub struct EditorMcpAssetsRoot(PathBuf);

impl EditorMcpAssetsRoot {
    /// Point QA-driven saves at this root.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self(root)
    }
}

impl Default for EditorMcpAssetsRoot {
    fn default() -> Self {
        Self(workspace_target())
    }
}

// A temp directory when the marker search fails, so a miss never writes inside the repo.
fn workspace_target() -> PathBuf {
    cobalt_ron_assets::workspace_assets_root().unwrap_or_else(std::env::temp_dir)
}
