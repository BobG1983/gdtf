//! Why a prefab write laid nothing down, as a typed refusal inside a successful reply.

use serde::{Deserialize, Serialize};

/// Which of the palette's two conditions a tile key failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum SelectTileRefusalNet {
    /// The terrain registry holds no def under that key, so the palette draws no row for it.
    NoTerrainDef,
    /// The session's theme does not list that terrain, so the palette never offers it.
    NotInTheThemePalette,
}

/// Why a paint wrote nothing before any placement rule was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum PaintRefusalNet {
    /// No tile is selected, so a click on the canvas lays nothing down either.
    NoSelectedTile,
}
