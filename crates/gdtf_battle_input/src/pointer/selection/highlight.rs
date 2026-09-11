//! Selection highlight sprite under the selected ganger.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, OccupancyGrid};

use crate::selection::resources::{SELECTION_TINT, SelectedShooter, SelectionHighlight};

/// Spawn or move the selection highlight to the selected ganger's cell.
pub fn update_selection_highlight(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    occupancy: Res<OccupancyGrid>,
    active_level: Res<ActiveLevel>,
    mut highlights: Query<(&mut Transform, &mut Visibility), With<SelectionHighlight>>,
) {
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
        Err(_) => {
            if let Some(world) = target {
                let sprite = Sprite {
                    color: SELECTION_TINT,
                    custom_size: Some(Vec2::splat(CELL_PX)),
                    ..default()
                };
                let transform = Transform::from_translation(world);
                let visibility = Visibility::Visible;
                let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
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

fn selected_cell(occupancy: &OccupancyGrid, level: Level, entity: Entity) -> Option<Cell> {
    use gdtf_battle_sim::{
        metric::CellUnit,
        occupancy::{GRID_HEIGHT, GRID_WIDTH, GridExtent},
    };
    for y in 0..*CellUnit::from(GridExtent::new(GRID_HEIGHT)) {
        for x in 0..*CellUnit::from(GridExtent::new(GRID_WIDTH)) {
            let cell = Cell::new(x, y);
            if occupancy.occupant(&CellLevel::new(cell, level)) == Some(entity) {
                return Some(cell);
            }
        }
    }
    None
}
