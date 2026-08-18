//! Which single-value field a write names, with the value on its own variant.

use serde::{Deserialize, Serialize};

use super::terrain_kind::TerrainKindNet;

/// One field of the active mode's draft, carrying the value it is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorFieldNet {
    /// The Terrain draft's kind pick.
    Kind(TerrainKindNet),
}
