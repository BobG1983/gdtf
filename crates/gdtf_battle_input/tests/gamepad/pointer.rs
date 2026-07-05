//! Software-cursor pointer arbitration: the picker honors `ActivePointer`, the
//! highlight follows the gamepad cursor, and the mouse reclaims (AC3/AC4).

use bevy::{
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    math::Vec2,
    prelude::*,
    transform::components::GlobalTransform,
    window::{CursorMoved, PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{ActivePointer, GamepadCursor, GdtfBattleInputPlugin, InspectTarget};
use gdtf_battle_presenter::{
    ActiveLevel, CellVisibility, HighlightRequest, ViewMode, WorldCamera, cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, CellLevel, OccupancyGrid, TerrainKind};
use gdtf_test_utils::{MessageProbe, MessageProbePlugin, probed};

use super::harness::*;

// =================================================================================
// AC3 — the picker honors `ActivePointer` (gamepad cursor vs OS cursor).
// =================================================================================

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds headlessly (the
/// `picking.rs` recipe): render-target info + a computed clip-from-view matrix.
fn synthetic_camera() -> Camera {
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(TARGET_SIZE.x, TARGET_SIZE.y);
    Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: TARGET_SIZE.as_uvec2(),
                scale_factor:  1.0,
            }),
            clip_from_view: projection.get_clip_from_view(),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    }
}

/// Builds a focused headless picking app (the `picking.rs` recipe): `MinimalPlugins` + the
/// input plugin, `ActiveLevel`, the `BattleInProgress` gate, a synthetic `WorldCamera` and
/// `Window`/`PrimaryWindow`. The OS cursor is left unset (off-window) so the gamepad path is
/// the only cursor source unless a test sets the OS cursor.
fn picking_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// Sets the primary window's OS cursor position in logical px (or clears it).
fn set_os_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Reads the current `InspectTarget` live hovered cell.
fn hovered(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
}

/// The camera's world unprojection of a screen `cursor` (so the test derives the expected
/// cell from the documented inverse instead of hardcoding the world math).
fn unproject(app: &mut App, cursor: Vec2) -> Option<Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
    let (camera, transform) = q.iter(app.world()).next()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

/// AC3 — with `ActivePointer::Gamepad` and a `GamepadCursor` over a known cell (resources set
/// directly), `pick_hovered_cell` writes THAT cell to `InspectTarget` (the gamepad cursor's,
/// NOT the OS cursor's); flipping back to `Mouse` reverts to the OS-cursor path.
#[test]
fn picker_honors_active_pointer() {
    let mut app = picking_app();

    // A gamepad-cursor screen point that lands on a non-origin in-grid cell (shifted right +
    // down from the centre — the `picking.rs` screen→world sign reasoning).
    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    // A DIFFERENT OS-cursor screen point (shifted further) so the two cursors disagree.
    let os_screen = TARGET_SIZE * 0.5 + Vec2::new(80.0, 64.0);
    set_os_cursor(&mut app, Some(os_screen));

    // Force the gamepad to be the active pointer + park its cursor over the known point.
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    // The expected cell is the documented inverse of the GAMEPAD cursor's unprojection.
    let gamepad_world = unproject(&mut app, gamepad_screen);
    let os_world = unproject(&mut app, os_screen);
    assert!(
        gamepad_world.is_some() && os_world.is_some(),
        "the synthetic camera must unproject both screen points",
    );
    let (Some(gamepad_world), Some(os_world)) = (gamepad_world, os_world) else {
        return;
    };
    let gamepad_cell = gdtf_battle_input::world_to_cell(gamepad_world, LEVEL);
    let os_cell = gdtf_battle_input::world_to_cell(os_world, LEVEL);
    assert!(
        gamepad_cell.is_some() && os_cell.is_some() && gamepad_cell != os_cell,
        "the two cursors must land on distinct in-grid cells for the arbitration to be \
         observable (gamepad {gamepad_cell:?}, os {os_cell:?})",
    );

    assert_eq!(
        hovered(&app),
        gamepad_cell,
        "in Gamepad mode the picker must project the GAMEPAD cursor, not the OS cursor",
    );

    // Flip back to Mouse -> the picker reverts to the OS-cursor path.
    app.world_mut().insert_resource(ActivePointer::Mouse);
    app.update();
    assert_eq!(
        hovered(&app),
        os_cell,
        "flipping to Mouse must revert the picker to the OS cursor",
    );
}

/// AC3 — the landed GTW-251 `emit_highlight_request` makes the highlight follow the gamepad
/// cursor for free: in Gamepad mode the emitted `HighlightRequest` equals `Some(the gamepad
/// cell)` — so the cell-highlight IS the cursor (no separate reticle).
#[test]
fn highlight_follows_the_gamepad_cursor() {
    let mut app = picking_app();
    // GTW-268 — the emit now gates on the `OccupancyGrid`; `picking_app` does not seed one,
    // so insert an empty grid here and (below) mark the resolved cell blocking.
    app.world_mut().insert_resource(OccupancyGrid::default());
    // The generic GTW-576 message probe — its `Last`-schedule drain observes the same
    // update's `emit_highlight_request` write with its own reader cursor.
    app.add_plugins(MessageProbePlugin::<HighlightRequest>::default());

    // OS cursor off-window; gamepad is the active pointer over a known cell.
    set_os_cursor(&mut app, None);
    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    let cell = hovered(&app);
    assert!(
        cell.is_some(),
        "the gamepad cursor must resolve an in-grid cell"
    );
    let Some(resolved) = cell else { return };

    // GTW-268 — the emit now highlights ONLY occupied / blocking cells. Make the gamepad's
    // resolved cell blocking (an object) so the highlight follows it; then drop the
    // bare-floor probe reads and re-run so the probe captures the post-gate emit.
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(resolved, TerrainKind::Cover);
    }
    if let Some(mut probe) = app
        .world_mut()
        .get_resource_mut::<MessageProbe<HighlightRequest>>()
    {
        probe.clear();
    }
    app.update();

    let emitted = probed::<HighlightRequest>(&app);
    assert_eq!(
        emitted,
        vec![HighlightRequest::new(cell, CellVisibility::NotSquadVisible)],
        "the highlight request must follow the gamepad cursor's resolved cell (GTW-11: \
         NotSquadVisible — no fog seeded, fail-closed)",
    );
    // Sanity: that cell really is `cell_to_world`-projectable (the highlight will draw there).
    if let Some(cell) = cell {
        let _ = cell_to_world(cell.cell(), LEVEL);
    }
}

// =================================================================================
// AC4 — the mouse reclaims the pointer on a CursorMoved message.
// =================================================================================

/// AC4 — with `ActivePointer::Gamepad`, emitting a `CursorMoved` message flips the active
/// pointer back to `Mouse` (last-moved-wins) via the REAL registered `mouse_reclaims_pointer`.
#[test]
fn mouse_reclaims_the_pointer() {
    let mut app = picking_app();
    // `MinimalPlugins` has no `InputPlugin`, so the `Messages<CursorMoved>` buffer is absent
    // (and `mouse_reclaims_pointer` is gated on it). Register it so the system runs and the
    // test can write the OS-cursor-moved message (under `DefaultPlugins` `InputPlugin`
    // provides this buffer).
    app.add_message::<CursorMoved>();
    // Start in Gamepad mode.
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.update();
    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Gamepad,
        "precondition: the pointer starts on Gamepad",
    );

    // Find the primary window entity to address the CursorMoved message at it.
    let window = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(app.world()).next()
    };
    assert!(
        window.is_some(),
        "the picking app must have a primary window"
    );
    let Some(window) = window else {
        return;
    };

    // Emit a CursorMoved (the OS mouse moved) and update — the mouse reclaims the pointer.
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<CursorMoved>>()
        .write(CursorMoved {
            window,
            position: Vec2::new(100.0, 100.0),
            delta: Some(Vec2::new(5.0, 5.0)),
        });
    app.update();

    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Mouse,
        "a CursorMoved message must flip the active pointer back to Mouse (last-moved-wins)",
    );
}
