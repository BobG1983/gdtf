//! Which way a tick in the theme's terrain library went.

use serde::{Deserialize, Serialize};

/// Whether a terrain toggle put the key into the theme's list or took it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum TerrainToggleNet {
    /// The theme did not hold that terrain, and holds it now.
    Added,
    /// The theme held that terrain, and does not hold it now.
    Removed,
}
