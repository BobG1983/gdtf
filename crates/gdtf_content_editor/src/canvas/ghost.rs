//! The hover-ghost overlay — [`spawn_hover_ghost`] and [`follow_hover_ghost`] `Update` systems
//! (swept onto the UUID-keyed terrain model in GTW-495).
//!
//! The ghost is a single persistent translucent [`CanvasGhost`] entity that snaps to the hovered
//! [`CanvasCell`], previewing the selected tile before it is painted.

use bevy::{
    prelude::*,
    ui::{PositionType, Val, widget::ImageNode},
};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{metric::CellLevel, terrain::def::TerrainDefRegistry};

use super::types::{CanvasCell, CanvasGhost, paint_index_for};
use crate::{
    editor_map::{EditorMap, GROUND_LEVEL},
    placement::{ProposedPlacement, evaluate_placement},
    session::MapEditorSession,
    tile_atlas::TileAtlas,
};

/// The reduced alpha a [`CanvasGhost`]'s preview sprite is tinted to, so it reads as a PREVIEW
/// distinct from a painted cell (GTW-426 C1). A framework presentation magnitude.
const GHOST_ALPHA: f32 = 0.55;

/// The partial-transparent RED a [`CanvasGhost`] is tinted when the hovered placement is ILLEGAL
/// (GTW-430 C2) — the visual reject signal the author sees before committing.
///
/// A named domain newtype (no-bare-types: a tint colour is a domain value, not a bare [`Color`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct GhostTint(Color);

impl GhostTint {
    /// The normal LEGAL preview tint — white at the reduced [`GHOST_ALPHA`].
    const fn legal() -> Self {
        Self(Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA))
    }

    /// The ILLEGAL tint — partial-transparent RED (GTW-430 C2): the cell reads as rejected.
    const fn illegal() -> Self {
        Self(Color::srgba(1.0, 0.2, 0.2, GHOST_ALPHA))
    }

    /// The tint for a placement, RED when illegal and the normal preview when legal.
    const fn for_illegal(is_illegal: bool) -> Self {
        if is_illegal {
            Self::illegal()
        } else {
            Self::legal()
        }
    }

    /// The wrapped [`Color`] to write onto the ghost [`ImageNode`].
    const fn color(self) -> Color {
        self.0
    }
}

/// `Update` (in `Editing`): spawn the single persistent hover-ghost overlay node (hidden) once.
///
/// An `Update` system (not `OnEnter`) because it reads the [`TileAtlas`], inserted via deferred
/// `Commands` on `OnEnter(Editing)`. Idempotent: it no-ops once a [`CanvasGhost`] exists, so it
/// spawns exactly one ghost the first frame the atlas is present.
pub(crate) fn spawn_hover_ghost(
    mut commands: Commands,
    atlas: Option<Res<TileAtlas>>,
    existing: Query<(), With<CanvasGhost>>,
) {
    let Some(atlas) = atlas else {
        return;
    };
    if !existing.is_empty() {
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
        .with_color(GhostTint::legal().color()),
        ghost_node(),
        // Above the cells (bevy-traps #8) so the preview paints over the cell fill.
        GlobalZIndex(1),
        Visibility::Hidden,
    ));
}

/// `Update` (in `Editing`): snap the hover-ghost to the hovered cell, or hide it (GTW-426 C1).
///
/// When a cell is hovered AND a tile is selected, it sets the ghost sprite to the SELECTED
/// terrain's atlas index (resolved THE WAY THE PRESENTER DOES) and re-parents the ghost under the
/// hovered cell (the snap), then shows it. GTW-430: before showing, it runs the SINGLE SHARED
/// legality predicate ([`evaluate_placement`]) for the hovered cell + selected tile; an ILLEGAL
/// verdict tints the ghost partial-transparent RED ([`GhostTint::illegal`]) — the same verdict the
/// click-commit rejects on (one source of truth, C3).
pub(crate) fn follow_hover_ghost(
    mut commands: Commands,
    registry: Option<Res<TerrainDefRegistry>>,
    roles: Option<Res<TileRoles>>,
    session: Option<Res<MapEditorSession>>,
    map: Option<Res<EditorMap>>,
    cells: Query<(Entity, &Interaction, &CanvasCell)>,
    mut ghost: Query<(Entity, &mut ImageNode, &mut Visibility), With<CanvasGhost>>,
) {
    let Ok((ghost_entity, mut ghost_node, mut visibility)) = ghost.single_mut() else {
        return;
    };
    let hovered = cells.iter().find_map(|(entity, interaction, cell)| {
        matches!(interaction, Interaction::Hovered).then_some((entity, cell.cell()))
    });
    let (Some(registry), Some(roles), Some(session), Some(map)) = (registry, roles, session, map)
    else {
        *visibility = Visibility::Hidden;
        return;
    };
    let selected = session.selected_tile();
    let selected_index =
        selected.and_then(|key| paint_index_for(&registry, &roles, session.theme(), key));
    let (Some((hovered_entity, hovered_cell)), Some(selected_key), Some(index)) =
        (hovered, selected, selected_index)
    else {
        // Not over a cell, or no tile selected — hide the ghost (C1).
        *visibility = Visibility::Hidden;
        return;
    };
    // The same predicate the commit uses (C3): preview the ground-plane slot for the hovered cell.
    let slot = CellLevel::new(hovered_cell, GROUND_LEVEL);
    let placement = ProposedPlacement::new(slot, selected_key);
    let verdict = evaluate_placement(
        &map,
        &registry,
        session.theme(),
        &placement,
        session.grid_size(),
    );
    if let Some(atlas) = ghost_node.texture_atlas.as_mut() {
        atlas.index = *index;
    }
    // Tint RED when the placement is illegal (C2), the normal preview when legal.
    ghost_node.color = GhostTint::for_illegal(verdict.is_illegal()).color();
    *visibility = Visibility::Visible;
    // Re-parent under the hovered cell so the absolute-positioned ghost snaps to it.
    commands
        .entity(ghost_entity)
        .insert(ChildOf(hovered_entity));
}

/// The hover-ghost overlay's [`Node`]: absolutely positioned to FILL its parent cell (inset 0 on
/// every edge), so once parented under the hovered [`CanvasCell`] it covers exactly that cell.
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
