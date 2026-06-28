//! The [`paint_cell`] `Update` system — clicking a canvas cell paints it with the selected tile.
//!
//! Separated from the type definitions ([`super::types`]) and the sync/ghost systems so the
//! click-to-paint logic has its own focused home.

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_sim::{level::ThemeCatalogRegistry, metric::CellLevel};

use super::types::{CanvasCell, paint_index_for};
use crate::{
    editor_map::{EditorMap, GROUND_LEVEL},
    placement::{ProposedPlacement, apply_placement},
    session::MapEditorSession,
};

/// The press-edge query filter [`paint_cell`] reads — a [`CanvasCell`] whose [`Interaction`]
/// changed this frame. A named alias to keep the system signature under clippy's
/// `type_complexity` gate (the `palette::PressedRow` precedent).
type PressedCell = (Changed<Interaction>, With<CanvasCell>);

/// `Update` (in `Editing`): a clicked canvas cell PAINTS it with the active palette tile — writes
/// the [`EditorMap`] model AND redraws the cell's fill sprite (GTW-426 C2 + C3).
///
/// Reads `Changed<Interaction> == Pressed` on the [`CanvasCell`] buttons (the engine's
/// `ui_focus_system` drives [`Interaction`] under `DefaultPlugins`; headless tests set it directly
/// then run `Update` — the GTW-422 `select_palette_tile` / dropdown-widget `press` precedent). On
/// a press, when a tile is SELECTED ([`MapEditorSession::selected_tile`]), it:
///
/// 1. runs the SINGLE SHARED legality predicate via
///    [`apply_placement`](crate::placement::apply_placement) — the SAME verdict the hover-ghost
///    previews (C3) — which REJECTS an illegal placement (leaving the [`EditorMap`] unchanged —
///    GTW-430 C2) and, on a legal one, performs any vertical auto-handling (placing a ladder
///    auto-clears a slab directly above it — GTW-430 C1) before recording the paint (clamped to
///    the drawable extent — C3), and
/// 2. mutates that cell's [`ImageNode`] atlas index IN PLACE to the painted tile's index (the
///    ui-mutate-not-respawn rule — never rebuild the whole grid for one paint), so the cell
///    immediately reflects the painted tile. Only when the placement COMMITTED — a rejected
///    (illegal) placement redraws nothing.
///
/// With no selected tile a click is a no-op (the contract: the ghost is hidden, nothing paints).
/// The canvas draws only the ground plane, so a click paints the `(cell, L0)` slot; the C1
/// auto-clear of a slab one storey up is a model-only effect (the 2D canvas has no L1 cell entity
/// to redraw — z is out of the canvas's scope).
pub(crate) fn paint_cell(
    pressed: Query<(Entity, &Interaction, &CanvasCell), PressedCell>,
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    map: Option<ResMut<EditorMap>>,
    mut cells: Query<&mut ImageNode, With<CanvasCell>>,
) {
    let (Some(registry), Some(session), Some(mut map)) = (registry, session, map) else {
        return;
    };
    let Some(selected) = session.selected_tile() else {
        // No active paint tile — a click paints nothing (the ghost is also hidden).
        return;
    };
    let Some(index) = paint_index_for(&registry, session.theme(), selected) else {
        // The selected tile has no resolvable sprite in the active theme — do not paint.
        return;
    };
    for (entity, interaction, cell) in &pressed {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        // Commit through the shared predicate (C3): rejects an illegal placement (C2), auto-clears
        // a slab above a placed ladder (C1), else records the paint. Only redraw on a commit.
        let slot = CellLevel::new(cell.cell(), GROUND_LEVEL);
        let placement = ProposedPlacement::new(slot, selected.clone());
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
