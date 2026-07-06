//! The selection-highlight sprite (GTW-225): [`update_selection_highlight`] keeps one reticle
//! snapped to the [`SelectedShooter`]'s cell on the presenter's active level.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, OccupancyGrid};

use crate::selection::resources::{
    SELECTION_TINT, SelectedShooter, SelectionHighlight, grid_extent_i32,
};

/// Maintains exactly ONE selection-highlight sprite that snaps to [`SelectedShooter`].
///
/// Spawns the single [`SelectionHighlight`] sprite the first time a ganger is selected; on every
/// later update it MOVES that one sprite's [`Transform`] to [`cell_to_world`] of the occupant's
/// cell (read off the [`OccupancyGrid`] occupant entry on the presenter's [`ActiveLevel`]) and
/// shows it, or HIDES it ([`Visibility::Hidden`]) when nothing is selected (or the selected
/// ganger is not on the active storey). Sized to one cell
/// (`custom_size: Some(Vec2::splat(CELL_PX))`) and drawn on
/// [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so it composites with the battlefield, not
/// the GTW-120 UI camera.
///
/// The selected ganger's CELL is found by scanning the [`OccupancyGrid`] on the active level for
/// the slot whose occupant is the selected entity — the presenter holds no sim→cell map, and the
/// occupancy grid is the authoritative entity→`(cell, level)` source (a read-only peek, never
/// mutated). No match on the active level → the highlight hides.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn, a
/// `Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>` for the move + show/hide,
/// `Res<SelectedShooter>` / `Res<OccupancyGrid>` / `Res<ActiveLevel>` reads. Runs
/// `.after(left_click_act)` so it reads the same update's selection.
pub fn update_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    occupancy: Res<OccupancyGrid>,
    active_level: Res<ActiveLevel>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>,
) {
    // The cell the selected ganger occupies on the active level (None when nothing is selected
    // or the selected ganger is on another storey), projected to world.
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
        // No highlight yet: spawn the single sprite the first time a ganger is selected on the
        // active level. (When nothing is selected there is nothing to spawn.)
        Err(_) => {
            if let Some(world) = target {
                let sprite = Sprite {
                    color: SELECTION_TINT,
                    custom_size: Some(Vec2::splat(CELL_PX)),
                    ..default()
                };
                let transform = Transform::from_translation(world);
                // Explicitly Visible (not the `Inherited` default) so the reticle shows from
                // the first frame, independent of parent visibility.
                let visibility = Visibility::Visible;
                let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
                // GTW-322 — authored as a `bsn!` scene (mirrors the presenter's converted
                // ganger sprite). The `Sprite` is NOT `Unpin` (its `Option<Handle<Image>>` /
                // `Option<TextureAtlas>` fields), so it rides NEITHER `template_value` (which
                // bounds `Unpin`) nor a `bsn!` field patch — it takes the
                // `template(move |_| Ok(value.clone()))` closure escape hatch (the `FnTemplate`
                // has no `Unpin` bound on its output). The `Transform` / `Visibility` /
                // `RenderLayers` ARE `Clone + Default + Unpin`, so each rides `template_value`.
                // The `SelectionHighlight` marker is a unit type, so it inlines in the macro.
                commands.spawn_scene((
                    bsn! {
                        SelectionHighlight
                        template(move |_| Ok(sprite.clone()))
                    },
                    template_value(transform),
                    template_value(visibility),
                    template_value(layers),
                ));
            }
        }
    }
}

/// The ground-plane [`Cell`] the `entity` occupies on `level`, by scanning the [`OccupancyGrid`]'s
/// slots for the one whose occupant is `entity`.
///
/// A read-only peek (never mutates the grid): the grid is the authoritative entity→`(cell, level)`
/// source and the presenter holds no sim→cell map. Returns [`None`] when `entity` is not occupying
/// any cell on `level` (e.g. it is on a different storey), so the highlight hides. Bounded by the
/// 60×60 grid extent.
fn selected_cell(occupancy: &OccupancyGrid, level: Level, entity: Entity) -> Option<Cell> {
    use gdtf_battle_sim::occupancy::{GRID_HEIGHT, GRID_WIDTH};
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
