//! What a destroyed terrain piece leaves standing in its cell.

use serde::{Deserialize, Serialize};

use super::TerrainUuid;
use crate::terrain::piece::TerrainGraphicKey;

/// What replaces a piece once it is destroyed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum LeavesBehind {
    /// Nothing stands here afterwards.
    #[default]
    Nothing,
    /// Another authored def, spawned with its own mechanics.
    Piece(TerrainUuid),
    /// A sprite with no mechanics at all.
    Sprite(TerrainGraphicKey),
}
