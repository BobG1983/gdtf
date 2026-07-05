//! Cursor-to-cell resolution + the fail-closed `None` paths (AC2/AC3).

use bevy::{math::Vec2, prelude::*};
use gdtf_battle_input::world_to_cell;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::Level;

use super::harness::*;

/// AC2 — given a synthesized camera + cursor, the picking maps the cursor to the
/// `CellLevel` the test computes INDEPENDENTLY from `viewport_to_world_2d` + the
/// documented inverse, and writes `InspectTarget(Some(..))`.
#[test]
fn picking_resolves_the_cursor_to_the_documented_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);

    // A cursor offset from the window centre so it lands on a non-origin in-grid cell.
    // Centre maps to world (0,0) = cell (0,0). An in-grid cell needs world.x >= 0 and
    // world.y <= 0; screen-y grows DOWNWARD while world-y grows UPWARD, so the cursor
    // shifts RIGHT (+screen x) and DOWN (+screen y) to reach the positive-x / positive-
    // row region.
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    // Compute the expected cell INDEPENDENTLY from the camera unprojection + the
    // documented inverse (cx = floor(world.x/CELL_PX), cy = floor(-world.y/CELL_PX)).
    let world = unproject(&mut app, cursor);
    assert!(
        world.is_some(),
        "the synthetic camera must unproject the cursor"
    );
    let Some(world) = world else { return };
    let expected = world_to_cell(world, level);
    assert!(
        expected.is_some(),
        "the chosen cursor must land inside the grid (world {world:?})",
    );

    assert_eq!(
        hovered(&app),
        expected,
        "InspectTarget must equal the documented inverse of the camera unprojection",
    );
}

/// AC3 — `InspectTarget` resolves fail-closed to `None` (no panic) when: the cursor is
/// off-window; the computed cell is outside the 60×60 grid; and there is no world
/// camera. Each case drives `app.update()` and asserts `None`.
#[test]
fn picking_fails_closed_to_none() {
    let level = Level::new(0);

    // (a) No cursor — `cursor_position()` is None.
    {
        let mut app = picking_app(level);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "an off-window cursor (no cursor_position) must resolve InspectTarget to None",
        );
    }

    // (b) A cursor whose world->cell lands OUTSIDE the grid. The window centre maps to
    // world (0,0) = cell (0,0); shifting LEFT of centre pushes world.x negative, so
    // cell.x floors below 0 — out of grid.
    {
        let mut app = picking_app(level);
        let cursor = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
        set_cursor(&mut app, Some(cursor));
        app.update();
        // Confirm the chosen cursor really is off-grid via the documented inverse.
        let world = unproject(&mut app, cursor);
        let Some(world) = world else {
            return;
        };
        assert_eq!(
            world_to_cell(world, level),
            None,
            "the chosen left-of-origin cursor must be off-grid (world {world:?})",
        );
        assert_eq!(
            hovered(&app),
            None,
            "an off-grid cursor must resolve InspectTarget to None",
        );
    }

    // (c) No world camera — despawn it, set an in-window cursor, update.
    {
        let mut app = picking_app(level);
        let cameras: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<WorldCamera>>();
            q.iter(app.world()).collect()
        };
        for camera in cameras {
            app.world_mut().entity_mut(camera).despawn();
        }
        set_cursor(&mut app, Some(TARGET_SIZE * 0.5));
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "with no WorldCamera the picking must resolve InspectTarget to None",
        );
    }
}
