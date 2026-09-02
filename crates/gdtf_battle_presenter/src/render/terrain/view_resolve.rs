//! Pick which of a terrain def's authored views the piece at a cell draws.

use bevy::prelude::*;
use gdtf_battle_sim::{
    def::{TerrainDef, TerrainDefRegistry, TerrainSimKind, TerrainTag, TerrainUuid, TerrainView},
    entity::TerrainCell,
    openable::OpenState,
    piece::TerrainGraphicKey,
    prelude::CellLevel,
    terrain::facing::TerrainFacing,
};

/// Which end of a vertical link a cell sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkEnd {
    /// The link's lower cell, where you ascend from.
    Lower,
    /// The link's upper cell, where you descend from.
    Upper,
}

// The view a def's tags and kind select, tags first so a door is not read as a wall.
fn view_for(
    def: &TerrainDef,
    facing: TerrainFacing,
    open: Option<OpenState>,
    link_end: Option<LinkEnd>,
) -> TerrainView {
    if def.tags.contains(&TerrainTag::Openable) {
        return match open {
            Some(OpenState::Open) => TerrainView::Open(facing),
            Some(OpenState::Closed) | None => TerrainView::Shut(facing),
        };
    }
    if def.tags.contains(&TerrainTag::Stair) {
        return match link_end {
            Some(LinkEnd::Upper) => TerrainView::FromAbove(facing),
            Some(LinkEnd::Lower) | None => TerrainView::FromBelow(facing),
        };
    }
    match def.sim_kind {
        TerrainSimKind::Wall { .. } => TerrainView::Edge(facing),
        TerrainSimKind::Cover { .. } | TerrainSimKind::Emplacement { .. } => {
            TerrainView::Facing(facing)
        }
        TerrainSimKind::Slab { .. } => TerrainView::Single,
    }
}

/// The sprite key this def names for the view its state and facing select.
///
/// A stair resolved with no link end answers its `FromBelow` row, the tile a static
/// draw and an editor thumbnail show.
#[must_use]
pub fn view_key_for(
    def: &TerrainDef,
    facing: TerrainFacing,
    open: Option<OpenState>,
    link_end: Option<LinkEnd>,
) -> Option<&TerrainGraphicKey> {
    def.views.sprite(view_for(def, facing, open, link_end))
}

/// Terrain pieces read by cell, def key, facing and open state, each carrying its own
/// change tick so a restamp can tell which rows moved.
pub(super) type TerrainPieces<'w, 's> = Query<
    'w,
    's,
    (
        &'static TerrainCell,
        Ref<'static, TerrainUuid>,
        Ref<'static, TerrainFacing>,
        Option<Ref<'static, OpenState>>,
    ),
>;

/// The sprite key the piece standing at `at` draws, resolved through its own def.
pub(super) fn view_key_at<'a>(
    at: CellLevel,
    pieces: &TerrainPieces,
    defs: &'a TerrainDefRegistry,
    link_end: Option<LinkEnd>,
) -> Option<&'a TerrainGraphicKey> {
    let (_, piece, facing, open) = pieces.iter().find(|(cell, ..)| ***cell == at)?;
    let def = defs.def(&piece)?;
    view_key_for(def, *facing, open.as_deref().copied(), link_end)
}

#[cfg(test)]
mod test;
