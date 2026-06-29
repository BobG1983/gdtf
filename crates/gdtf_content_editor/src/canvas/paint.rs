//! The [`paint_cell`] `Update` system — clicking a canvas cell paints it with the selected tile
//! (swept onto the UUID-keyed terrain model in GTW-495).

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{metric::CellLevel, terrain::def::TerrainDefRegistry};

use super::types::{CanvasCell, paint_index_for};
use crate::{
    editor_map::{EditorMap, GROUND_LEVEL},
    placement::{ProposedPlacement, apply_placement},
    session::MapEditorSession,
};

/// The press-edge query filter [`paint_cell`] reads — a [`CanvasCell`] whose [`Interaction`]
/// changed this frame. A named alias to keep the system signature under clippy's
/// `type_complexity` gate.
type PressedCell = (Changed<Interaction>, With<CanvasCell>);

/// `Update` (in `Editing`): a clicked canvas cell PAINTS it with the active palette tile — writes
/// the [`EditorMap`] model AND redraws the cell's fill sprite (GTW-426 C2 + C3).
///
/// On a press, when a tile is SELECTED ([`MapEditorSession::selected_tile`]), it:
///
/// 1. runs the SINGLE SHARED legality predicate via [`apply_placement`] — the SAME verdict the
///    hover-ghost previews (C3) — which REJECTS an illegal placement (GTW-430 C2) and, on a legal
///    one, performs any vertical auto-handling (GTW-430 C1) before recording the paint, and
/// 2. mutates that cell's [`ImageNode`] atlas index IN PLACE to the painted terrain's index
///    (resolved THE WAY THE PRESENTER DOES, via the def's graphic against [`TileRoles`]).
///
/// With no selected tile a click is a no-op.
pub(crate) fn paint_cell(
    pressed: Query<(Entity, &Interaction, &CanvasCell), PressedCell>,
    registry: Option<Res<TerrainDefRegistry>>,
    roles: Option<Res<TileRoles>>,
    session: Option<Res<MapEditorSession>>,
    map: Option<ResMut<EditorMap>>,
    mut cells: Query<&mut ImageNode, With<CanvasCell>>,
) {
    let (Some(registry), Some(roles), Some(session), Some(mut map)) =
        (registry, roles, session, map)
    else {
        return;
    };
    let Some(selected) = session.selected_tile() else {
        // No active paint tile — a click paints nothing (the ghost is also hidden).
        return;
    };
    let Some(index) = paint_index_for(&registry, &roles, session.theme(), selected) else {
        // The selected terrain has no resolvable sprite in the active theme — do not paint.
        return;
    };
    for (entity, interaction, cell) in &pressed {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        // Commit through the shared predicate (C3): rejects an illegal placement (C2), auto-clears
        // a slab above a placed ladder (C1), else records the paint. Only redraw on a commit.
        let slot = CellLevel::new(cell.cell(), GROUND_LEVEL);
        let placement = ProposedPlacement::new(slot, selected);
        if apply_placement(
            &mut map,
            &registry,
            session.theme(),
            &placement,
            session.grid_size(),
        ) && let Ok(mut node) = cells.get_mut(entity)
            && let Some(atlas) = node.texture_atlas.as_mut()
        {
            atlas.index = *index;
        }
    }
}
