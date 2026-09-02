//! The authored art a terrain def carries, one row per view it can be drawn in.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use crate::terrain::{
    facing::{TerrainCorner, TerrainFacing},
    piece::TerrainGraphicKey,
};

/// One view a terrain piece can be drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum TerrainView {
    /// A wall's straight run along one side.
    Edge(TerrainFacing),
    /// A wall's turn at one corner.
    Corner(TerrainCorner),
    /// A cover or emplacement piece seen from one side.
    Facing(TerrainFacing),
    /// A door standing shut, on one side.
    Shut(TerrainFacing),
    /// A door standing open, on one side.
    Open(TerrainFacing),
    /// A staircase seen from the storey below, on one side.
    FromBelow(TerrainFacing),
    /// A staircase seen from the storey above, on one side.
    FromAbove(TerrainFacing),
    /// The one view a piece that owes a single drawing carries.
    Single,
}

/// One authored row: the view, and the sprite def that draws it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct TerrainViewArt {
    /// Which view this row draws.
    pub view:   TerrainView,
    /// The sprite def key, by file stem.
    pub sprite: TerrainGraphicKey,
}

/// Every art row a terrain def authors.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct TerrainViews(Vec<TerrainViewArt>);

impl TerrainViews {
    /// Wrap the authored rows.
    #[must_use]
    pub const fn new(rows: Vec<TerrainViewArt>) -> Self {
        Self(rows)
    }

    /// The sprite key this def names for one view, if it names one.
    #[must_use]
    pub fn sprite(&self, view: TerrainView) -> Option<&TerrainGraphicKey> {
        self.0
            .iter()
            .find(|row| row.view == view)
            .map(|row| &row.sprite)
    }
}
