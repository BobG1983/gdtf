//! The painted slots of one storey, as a client reads them back.

use bevy::prelude::Deref;
use gdtf_battle_sim::metric::CellLevel;
use serde::{Deserialize, Serialize};

use super::{cell::EditorCellLevelNet, facing::TerrainFacingNet, key::TerrainKeyNet};
use crate::editor_map::PaintedPiece;

/// One painted slot: where it is, what is in it, and which way that is turned.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct PaintedRowNet {
    /// The slot the piece occupies.
    cell:   EditorCellLevelNet,
    /// The terrain def painted there.
    tile:   TerrainKeyNet,
    /// Which way that piece is turned.
    facing: TerrainFacingNet,
}

impl PaintedRowNet {
    /// Mirror one entry of the editor's own painted map.
    pub(in crate::net_qa) fn from_piece(slot: CellLevel, piece: PaintedPiece) -> Self {
        Self {
            cell:   EditorCellLevelNet::from_slot(slot),
            tile:   TerrainKeyNet::new((*piece.tile()).to_string()),
            facing: TerrainFacingNet::from_facing(piece.facing()),
        }
    }
}

/// Every painted slot a read answered with, in the order it answered them.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct PaintedMapNet(Vec<PaintedRowNet>);

impl PaintedMapNet {
    /// Wrap the rows a read collected.
    #[must_use]
    pub(in crate::net_qa) const fn new(rows: Vec<PaintedRowNet>) -> Self {
        Self(rows)
    }
}
