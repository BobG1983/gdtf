//! Ganger selection (GTW-225 / GTW-48 S8): the [`SelectedShooter`] resource set by
//! left-clicking an occupied cell, and the [`SelectionHighlight`] sprite that snaps
//! to the selected cell.
//!
//! Left-click reads the [`HoveredCell`](crate::HoveredCell) (the S7 picking result)
//! and the sim's [`OccupancyGrid::occupant`](gdtf_battle_sim::OccupancyGrid::occupant)
//! at that cell: an occupied cell selects its occupant, an empty cell (or no hover)
//! clears the selection. The highlight reuses the S7
//! [`update_hover_highlight`](crate::update_hover_highlight) one-sprite recipe
//! verbatim (single sprite, spawn-once + [`Visibility`] toggle,
//! [`cell_to_world`](gdtf_battle_presenter::cell_to_world) +
//! [`CELL_PX`](gdtf_battle_presenter::CELL_PX), on the
//! [`WORLD_RENDER_LAYER`](gdtf_battle_presenter::WORLD_RENDER_LAYER)).
//!
//! # Faction-agnostic selection (DEFERRED gate)
//!
//! This slice selects ANY occupied ganger — there is NO `Faction` filter and NO
//! `Faction(0) == player` literal anywhere. No player-identity concept exists in the
//! codebase yet (`Faction(u8)` is a gang-agnostic index defaulting to 0; the
//! skirmish authors two symmetric sides), so own-ganger-only is DEFERRED to GTW-226
//! (player identity), per the ticket's recorded user decision. When that lands it
//! adds a `PlayerFaction` resource and gates [`select_on_click`] on it; until then,
//! clicking any occupied cell selects its occupant.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::{Cell, Level, OccupancyGrid};

use crate::HoveredCell;

/// The currently SELECTED shooter — the ganger a left-click picked, or `None`.
///
/// A named newtype over `Option<Entity>` (no-bare-types: the selection is a domain
/// value; [`Entity`] is the framework carve-out) that [`Deref`]s to its inner
/// [`Option`] so a reader matches it directly. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) — so its [`Default`] is
/// the empty selection (`None`). [`select_on_click`] writes it; the
/// [`update_selection_highlight`] sprite + the act surfaces (222b/222c) read it.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedShooter(pub Option<Entity>);

impl SelectedShooter {
    /// Build a selection holding `entity`.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// The empty (no-ganger) selection — what a click on an empty cell, or a
    /// [`SelectionClear`](crate::ActIntent::SelectionClear) intent, sets.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// Marker for the single selection-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the
/// same justification the S7 `HoverHighlight` / the presenter's `WorldCamera`
/// markers use): [`update_selection_highlight`] queries `With<SelectionHighlight>`
/// to find and MOVE the one existing highlight rather than spawning a duplicate each
/// update. Distinct from the hover highlight so the two reticles coexist.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SelectionHighlight;

/// The translucent tint of the selection-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a
/// domain quantity (the `CELL_PX`-class const carve-out, the S7 `HIGHLIGHT_TINT`
/// precedent). A cooler cyan at moderate alpha so the SELECTED cell reads distinctly
/// from the warm-white hover reticle when both are on the same cell.
const SELECTION_TINT: Color = Color::srgba(0.4, 0.85, 1.0, 0.5);

/// Sets [`SelectedShooter`] from a left-click on the [`HoveredCell`].
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Left)`, reads the [`HoveredCell`]
/// (the S7 picking result) and the [`OccupancyGrid`] occupant at that cell:
///
/// - an occupied cell → select its occupant ([`SelectedShooter::new`]),
/// - an empty cell, OR no cell hovered → clear ([`SelectedShooter::cleared`]).
///
/// FACTION-AGNOSTIC: it selects ANY occupant — there is no `Faction` read or filter
/// (own-ganger-only is deferred to GTW-226; see the module docs). Writes only on a
/// real change (change-detection hygiene). Param-only (`bevy-traps.md` #7):
/// `Res<ButtonInput<MouseButton>>` + `Res<HoveredCell>` + `Res<OccupancyGrid>`
/// reads, `ResMut<SelectedShooter>` write — no `&mut World`.
pub fn select_on_click(
    mouse: Res<ButtonInput<MouseButton>>,
    hovered: Res<HoveredCell>,
    occupancy: Res<OccupancyGrid>,
    mut selected: ResMut<SelectedShooter>,
) {
    // Only act on the press edge; a held button does not re-select.
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // The occupant at the hovered cell (None when nothing is hovered or the cell is
    // empty) — faction-agnostic: any occupant is selectable.
    let picked = (**hovered).and_then(|cell| occupancy.occupant(&cell));
    let next = match picked {
        Some(entity) => SelectedShooter::new(entity),
        None => SelectedShooter::cleared(),
    };
    if *selected != next {
        *selected = next;
    }
}

/// Maintains exactly ONE selection-highlight sprite that snaps to [`SelectedShooter`].
///
/// The S7 [`update_hover_highlight`](crate::update_hover_highlight) recipe applied to
/// the SELECTED ganger: spawns the single [`SelectionHighlight`] sprite the first
/// time a ganger is selected; on every later update it MOVES that one sprite's
/// [`Transform`] to [`cell_to_world`] of the occupant's cell (read off the
/// [`OccupancyGrid`] occupant entry on the presenter's [`ActiveLevel`]) and shows it,
/// or HIDES it ([`Visibility::Hidden`]) when nothing is selected (or the selected
/// ganger is not on the active storey). Sized to one cell
/// (`custom_size: Some(Vec2::splat(CELL_PX))`) and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the
/// battlefield, not the GTW-120 UI camera.
///
/// The selected ganger's CELL is found by scanning the [`OccupancyGrid`] on the
/// active level for the slot whose occupant is the selected entity — the presenter
/// holds no sim→cell map, and the occupancy grid is the authoritative
/// entity→`(cell, level)` source (a read-only peek, never mutated). No match on the
/// active level → the highlight hides (the selected ganger is on another storey).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn, a
/// `Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>` for the move +
/// show/hide, `Res<SelectedShooter>` / `Res<OccupancyGrid>` / `Res<ActiveLevel>`
/// reads. Runs `.after(select_on_click)` so it reads the same update's selection.
pub fn update_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    occupancy: Res<OccupancyGrid>,
    active_level: Res<ActiveLevel>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>,
) {
    // The cell the selected ganger occupies on the active level (None when nothing is
    // selected or the selected ganger is on another storey), projected to world.
    let target = (**selected)
        .and_then(|entity| selected_cell(&occupancy, **active_level, entity))
        .map(|cell| cell_to_world(cell, **active_level));

    match highlights.single_mut() {
        Ok((mut transform, mut visibility)) => match target {
            Some(world) => {
                transform.translation = world;
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        },
        // No highlight yet: spawn the single sprite the first time a ganger is
        // selected on the active level. (When nothing is selected there is nothing to
        // spawn — it stays absent until the first selection, equivalent to hidden.)
        Err(_) => {
            if let Some(world) = target {
                commands.spawn((
                    SelectionHighlight,
                    Sprite {
                        color: SELECTION_TINT,
                        custom_size: Some(Vec2::splat(CELL_PX)),
                        ..default()
                    },
                    Transform::from_translation(world),
                    // Explicitly Visible (not the `Inherited` default) so the reticle
                    // shows from the first frame, independent of parent visibility.
                    Visibility::Visible,
                    RenderLayers::layer(WORLD_RENDER_LAYER),
                ));
            }
        }
    }
}

/// The ground-plane [`Cell`] the `entity` occupies on `level`, by scanning the
/// [`OccupancyGrid`]'s slots for the one whose occupant is `entity`.
///
/// A read-only peek (never mutates the grid): the grid is the authoritative
/// entity→`(cell, level)` source and the presenter holds no sim→cell map. Returns
/// [`None`] when `entity` is not occupying any cell on `level` (e.g. it is on a
/// different storey), so the highlight hides. Bounded by the 60×60 grid extent.
fn selected_cell(occupancy: &OccupancyGrid, level: Level, entity: Entity) -> Option<Cell> {
    use gdtf_battle_sim::{CellLevel, GRID_HEIGHT, GRID_WIDTH};
    for y in 0..grid_extent_i32(GRID_HEIGHT) {
        for x in 0..grid_extent_i32(GRID_WIDTH) {
            let cell = Cell::new(x, y);
            if occupancy.occupant(&CellLevel::new(cell, level)) == Some(entity) {
                return Some(cell);
            }
        }
    }
    None
}

/// Widens a `usize` grid extent to `i32` for the [`selected_cell`] scan bounds.
///
/// 60 fits `i32` comfortably, but `as i32` on a `usize` trips `cast_possible_wrap`
/// (`-D`); [`i32::try_from`] is the no-`unwrap` cast — the S7 `grid_extent_i32`
/// precedent. An unrepresentable extent saturates to [`i32::MAX`] (only widens the
/// scan, never narrows it).
fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
