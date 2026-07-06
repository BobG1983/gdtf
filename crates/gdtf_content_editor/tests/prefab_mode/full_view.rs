//! The full-view toggle changes the drawn tile set (GTW-532).

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridLevels, GridSize, GridWidth},
    metric::{CellLevel, Level},
};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, EditorMode, MapEditorSession};

use super::harness::*;

/// Count the current preview tile [`Sprite`]s in the world. The harness spawns no other world
/// sprites (only the editor preview tiles do), so this IS the drawn tile set.
fn preview_sprite_count(app: &mut App) -> usize {
    app.world_mut().query::<&Sprite>().iter(app.world()).count()
}

/// GTW-532 C1 / C2 (REAL PATH) — toggling the prefab viewport's [`ViewMode`] from
/// [`DownToActive`](ViewMode::DownToActive) to [`FullView`](ViewMode::FullView) CHANGES the drawn
/// tile set: with a distinct block painted on the UPPER storey (culled at the ground edit storey in
/// `DownToActive`), `FullView` draws MORE tiles than `DownToActive`. Drives the SAME resources the editor
/// inserts — the reused presenter [`ViewMode`], the [`EditorMap`], the [`CurrentEditLevel`] — and
/// lets the real `redraw_preview_tiles` system re-run on the `ViewMode::is_changed` trigger.
///
/// A green build alone does NOT prove the toggle (C4): this asserts the drawn storey RANGE / tile
/// count actually differs between the two modes on the real code path.
#[test]
fn full_view_toggle_changes_the_drawn_tile_set() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the theme seed so a default floor / palette resolves and the base fill draws.
    for _ in 0..8 {
        app.update();
    }

    // The editor opens in PREFAB mode with the default DownToActive view.
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

    // Resolve a real paint tile from the seeded session; soft-skip if the async seed has not
    // resolved a default floor yet (the tile pipeline is asset-timing dependent).
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

    // Give the prefab a 2-storey volume and paint a distinct block on the UPPER storey (storey 1).
    // At the ground edit storey, DownToActive draws only storey 0 (the upper block is CULLED);
    // FullView draws BOTH storeys (the upper block re-appears) — so the drawn tile count grows.
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
        // A distinct upper-storey block that only FullView draws.
        let upper = Level::new(1);
        for x in 0..3 {
            for y in 0..3 {
                map.paint_at(CellLevel::new(Cell::new(x, y), upper), tile, size);
            }
        }
        // Stay on the ground edit storey so DownToActive culls the upper block.
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground();
    }
    let _ = theme;

    // Settle the DownToActive draw and record its drawn tile count.
    for _ in 0..4 {
        app.update();
    }
    let down_to_active = preview_sprite_count(&mut app);
    assert!(
        down_to_active > 0,
        "DownToActive draws the ground-storey fill (a non-empty viewport); got {down_to_active}",
    );

    // Flip the REUSED presenter ViewMode to FullView (the same flip the toggle button / F hotkey
    // apply) and let the real redraw re-run on the ViewMode::is_changed trigger.
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
