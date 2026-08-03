use bevy::prelude::*;
use gdtf_battle_presenter::{IsolateView, ViewMode};
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
        .filter(|(sprite, transform)| {
            sprite.rect.is_some() && (transform.translation.z - target_z).abs() < Z_TOLERANCE
        })
        .count()
}

#[test]
fn default_floor_fill_is_anchored_to_the_ground_storey() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    assert_eq!(
        app.world().get_resource::<ViewMode>().copied(),
        Some(ViewMode::DownToActive),
        "the prefab viewport opens in the default DownToActive view",
    );
    {
        let world = app.world_mut();
        let Some(mut isolate) = world.get_resource_mut::<IsolateView>() else {
            unreachable!("isolate toggle inserted in Editing");
        };
        *isolate = IsolateView::Off;
    }

    let has_default_floor = app
        .world()
        .get_resource::<MapEditorSession>()
        .and_then(MapEditorSession::default_floor)
        .is_some();
    if !has_default_floor {
        assert!(
            app.world().get_resource::<EditorMap>().is_some(),
            "map inserted in Editing",
        );
        return;
    }

    let width;
    let height;
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
        width = usize::from(*size.width());
        height = usize::from(*size.height());

        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        let lifted = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), size)
            .stepped(LevelStep::up(), size);
        assert_eq!(
            *lifted.level(),
            2,
            "the edit cursor is lifted to the upper storey 2",
        );
        *edit_level = lifted;
    }

    for _ in 0..4 {
        app.update();
    }

    let ground_fill = sprite_count_on_storey(&mut app, 0);
    let upper_fill = sprite_count_on_storey(&mut app, 2);
    let expected_cells = width * height;

    assert_eq!(
        ground_fill, expected_cells,
        "the default-floor fill must be spawned on the GROUND storey (storey 0, z≈0) — one sprite \
         per cell — even when the edit cursor is on an upper storey (GTW-535); expected \
         {expected_cells}, got {ground_fill}",
    );

    assert_eq!(
        upper_fill, 0,
        "the unpainted ACTIVE upper storey (storey 2, z≈2·gap) must draw NO default-floor fill — \
         the fill does not follow the edit cursor up the stack (GTW-535); got {upper_fill}",
    );
}
