use bevy::prelude::*;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, LevelStep, MapEditorSession};

use super::harness::*;

const STOREY_Z_GAP: f32 = 0.01;

const Z_TOLERANCE: f32 = STOREY_Z_GAP / 2.0;

fn sprite_count_on_storey(app: &mut App, storey: u8) -> usize {
    let target_z = STOREY_Z_GAP * f32::from(storey);
    app.world_mut()
        .query::<(&Sprite, &Transform)>()
        .iter(app.world())
        .filter(|(_, transform)| (transform.translation.z - target_z).abs() < Z_TOLERANCE)
        .count()
}

#[test]
fn editor_default_isolates_the_active_storey_with_one_onion_below() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }
    let tile = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            assert!(
                world.get_resource::<EditorMap>().is_some(),
                "map inserted in Editing"
            );
            return;
        };
        tile
    };

    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Ok(size) = GridSize::new(GridWidth::new(6), GridHeight::new(6), GridLevels::new(3))
        else {
            unreachable!("a 6×6×3 grid is valid");
        };
        session.set_grid_size(size);
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        map.paint_at(
            gdtf_battle_sim::metric::CellLevel::new(
                gdtf_battle_sim::prelude::Cell::new(2, 2),
                gdtf_battle_sim::metric::Level::new(1),
            ),
            tile,
            size,
        );
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), size)
            .stepped(LevelStep::up(), size);
    }
    for _ in 0..4 {
        app.update();
    }

    let ground = sprite_count_on_storey(&mut app, 0);
    let onion = sprite_count_on_storey(&mut app, 1);
    let active = sprite_count_on_storey(&mut app, 2);

    assert_eq!(
        ground, 0,
        "the editor's DEFAULT (Isolate) view at edit storey 2 must draw NOTHING on storey 0 — \
         the whole-stack DownToActive draw is the symptom; got {ground} \
         sprite(s) on storey 0",
    );
    assert!(
        active > 0,
        "the ACTIVE edit storey (2) must draw content in the Isolate view (the void grid on \
         unpainted cells); got {active}",
    );
    assert!(
        onion > 0,
        "the ONE onion storey below the active (1) must draw its painted cell as the \
         categorical below-ghost; got {onion}",
    );
}
