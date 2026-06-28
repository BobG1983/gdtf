//! The hover-ghost overlay — [`spawn_hover_ghost`] and [`follow_hover_ghost`] `Update` systems.
//!
//! The ghost is a single persistent translucent [`CanvasGhost`] entity that snaps to the hovered
//! [`CanvasCell`], previewing the selected tile before it is painted. Separated from the type
//! definitions ([`super::types`]) and the sync/paint systems so the ghost lifecycle has its own
//! focused home.

use bevy::{
    prelude::*,
    ui::{PositionType, Val, widget::ImageNode},
};
use gdtf_battle_sim::level::ThemeCatalogRegistry;

use super::types::{CanvasCell, CanvasGhost, paint_index_for};
use crate::{session::MapEditorSession, tile_atlas::TileAtlas};

/// The reduced alpha a [`CanvasGhost`]'s preview sprite is tinted to, so it reads as a PREVIEW
/// distinct from a painted cell (GTW-426 C1). A framework presentation magnitude (the clause-4
/// plumbing carve-out, the `canvas::dimmed` alpha-halving precedent), fed to the ghost
/// [`ImageNode`]'s color.
const GHOST_ALPHA: f32 = 0.55;

/// `Update` (in `Editing`): spawn the single persistent hover-ghost overlay node (hidden) once,
/// the preview sprite the hover-ghost flow snaps to the hovered cell (GTW-426 C1).
///
/// An `Update` system (not `OnEnter`) because it reads the [`TileAtlas`], inserted via deferred
/// `Commands` on `OnEnter(Editing)` — an `OnEnter` spawn chained after the atlas insert would NOT
/// see it (the GTW-421 / GTW-422 command-flush race; the `sync_canvas` / `sync_palette` precedent).
/// Idempotent: it no-ops once a [`CanvasGhost`] exists, so it spawns exactly one ghost the first
/// frame the atlas is present.
///
/// ONE [`CanvasGhost`] entity (the ui-mutate rule: re-parented + retinted in place per hover,
/// never respawned). It starts [`Visibility::Hidden`] — [`follow_hover_ghost`] shows it, sets its
/// sprite to the selected tile, and re-parents it under the hovered cell. A [`GlobalZIndex`]
/// strictly above the cells (bevy-traps #8) so the translucent preview paints OVER the cell fill
/// rather than behind it. Spawned detached (no parent); the follow system parents it under the
/// hovered cell each frame.
pub(crate) fn spawn_hover_ghost(
    mut commands: Commands,
    atlas: Option<Res<TileAtlas>>,
    existing: Query<(), With<CanvasGhost>>,
) {
    let Some(atlas) = atlas else {
        return;
    };
    if !existing.is_empty() {
        // The ghost already exists — spawn exactly one (the ui-mutate rule).
        return;
    }
    commands.spawn((
        CanvasGhost,
        ImageNode::from_atlas_image(
            atlas.image(),
            TextureAtlas {
                layout: atlas.layout(),
                index:  0,
            },
        )
        .with_color(Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA)),
        ghost_node(),
        // Above the cells (bevy-traps #8) so the preview paints over the cell fill.
        GlobalZIndex(1),
        Visibility::Hidden,
    ));
}

/// `Update` (in `Editing`): snap the hover-ghost to the hovered cell, or hide it (GTW-426 C1).
///
/// Drives the ghost off the [`CanvasCell`] the engine's `ui_focus_system` marked
/// [`Interaction::Hovered`] (bevy-traps #6 — under `DefaultPlugins` real mouse hover already sets
/// this; headless tests set it directly + run `Update`). When a cell is hovered AND a tile is
/// selected, it:
///
/// 1. sets the ghost sprite to the SELECTED tile's atlas index (so the preview shows what would be
///    painted), and
/// 2. re-parents the ghost under the hovered cell — an absolute-positioned node filling the cell,
///    so it SNAPS to (and tracks the layout of) the hovered cell for free — and shows it.
///
/// When NO cell is hovered, or NO tile is selected, the ghost is hidden (C1). Mutates the one
/// persistent ghost in place (the ui-mutate rule); never respawns it.
pub(crate) fn follow_hover_ghost(
    mut commands: Commands,
    registry: Option<Res<ThemeCatalogRegistry>>,
    session: Option<Res<MapEditorSession>>,
    cells: Query<(Entity, &Interaction), With<CanvasCell>>,
    mut ghost: Query<(Entity, &mut ImageNode, &mut Visibility), With<CanvasGhost>>,
) {
    let Ok((ghost_entity, mut ghost_node, mut visibility)) = ghost.single_mut() else {
        return;
    };
    let hovered = cells.iter().find_map(|(entity, interaction)| {
        matches!(interaction, Interaction::Hovered).then_some(entity)
    });
    let selected_index = match (registry, session) {
        (Some(registry), Some(session)) => session
            .selected_tile()
            .and_then(|key| paint_index_for(&registry, session.theme(), key)),
        _ => None,
    };
    let (Some(hovered), Some(index)) = (hovered, selected_index) else {
        // Not over a cell, or no tile selected — hide the ghost (C1).
        *visibility = Visibility::Hidden;
        return;
    };
    if let Some(atlas) = ghost_node.texture_atlas.as_mut() {
        atlas.index = *index;
    }
    *visibility = Visibility::Visible;
    // Re-parent under the hovered cell so the absolute-positioned ghost snaps to it.
    commands.entity(ghost_entity).insert(ChildOf(hovered));
}

/// The hover-ghost overlay's [`Node`]: absolutely positioned to FILL its parent cell (inset 0 on
/// every edge), so once parented under the hovered [`CanvasCell`] it covers exactly that cell —
/// the snap. Absolute so it sits over the cell's content without disturbing the grid flex layout.
fn ghost_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        right: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}
