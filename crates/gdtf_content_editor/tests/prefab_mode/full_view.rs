use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{CellLevel, Level},
    prelude::Cell,
};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, EditorMode, MapEditorSession};

use super::harness::*;

fn preview_sprite_count(app: &mut App) -> usize {
    app.world_mut().query::<&Sprite>().iter(app.world()).count()
}

#[test]
fn full_view_toggle_changes_the_drawn_tile_set() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in PREFAB mode",
    );
    assert_eq!(
        app.world().get_resource::<ViewMode>().copied(),
        Some(ViewMode::DownToActive),
        "the prefab viewport opens in the default DownToActive view (GTW-532 C1)",
    );
    assert_eq!(
        app.world().get_resource::<IsolateView>().copied(),
        Some(IsolateView::On(ContextDepth::new(1))),
        "the editor opens with the GTW-594 Isolate default (one onion storey below)",
    );
    {
        let world = app.world_mut();
        let Some(mut isolate) = world.get_resource_mut::<IsolateView>() else {
            unreachable!("isolate toggle inserted in Editing");
        };
        *isolate = IsolateView::Off;
    }

    let (tile, theme) = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            assert!(
                world.get_resource::<ViewMode>().is_some(),
                "view mode inserted"
            );
            return;
        };
        (tile, session.theme())
    };

    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        if let Ok(size) = GridSize::new(GridWidth::new(6), GridHeight::new(6), GridLevels::new(2)) {
            session.set_grid_size(size);
        }
        let size = session.grid_size();
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        let upper = Level::new(1);
        for x in 0..3 {
            for y in 0..3 {
                map.paint_at(CellLevel::new(Cell::new(x, y), upper), tile, size);
            }
        }
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground();
    }
    let _ = theme;

    for _ in 0..4 {
        app.update();
    }
    let down_to_active = preview_sprite_count(&mut app);
    assert!(
        down_to_active > 0,
        "DownToActive draws the ground-storey fill (a non-empty viewport); got {down_to_active}",
    );

    {
        let world = app.world_mut();
        let Some(mut view) = world.get_resource_mut::<ViewMode>() else {
            unreachable!("view mode inserted in Editing");
        };
        *view = ViewMode::FullView;
    }
    for _ in 0..4 {
        app.update();
    }
    let full_view = preview_sprite_count(&mut app);

    assert!(
        full_view > down_to_active,
        "toggling to FullView must draw MORE tiles than DownToActive — the upper-storey block \
         re-appears (the mode switch changes the drawn storey range / tile set, GTW-532 C2); \
         DownToActive={down_to_active}, FullView={full_view}",
    );
}
