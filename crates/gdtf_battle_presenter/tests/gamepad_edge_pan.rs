//! GTW-259 / GTW-271 (AC5 / AC4): the GAMEPAD-cursor edge-pan through the REAL registered
//! `pan_camera_on_gamepad_cursor_edge`, headless.
//!
//! GTW-271 reworked this edge-pan to key off the MAP VIEWPORT rect
//! (`Camera::logical_viewport_rect`) instead of the whole window: it pans only when the gamepad
//! cursor is INSIDE that rect and within the edge band. `logical_viewport_rect()` returns `None`
//! until Bevy's `camera_system` computes the camera's render-target info — which never runs
//! under `MinimalPlugins` — so headlessly the gate fail-closes and the system does NOT pan. The
//! deterministic edge→direction logic (cursor inside the viewport near an edge → a camera
//! direction, with the screen-y → camera-y flip) is therefore unit-tested directly on the pure
//! `viewport_edge_dir` helper (`world_camera::test`); the full pan-with-a-live-viewport is
//! in-engine QA / TBD (Bevy harness) (`verification.md` #3, the contract's honesty clause).
//!
//! What these headless tests still pin, on the REAL registered system (battle-gated `Update`,
//! `.before(clamp_camera_to_bounds)`, not a copy): (1) the edge-pan is VIEWPORT-GATED — with no
//! viewport rect (the headless reality) even an edge-band `GamepadCursorMoved` produces NO pan
//! (the GTW-271 fail-closed gate); (2) the edge-pan is MESSAGE-driven — no message → no pan.
//! Every `app.world_mut()` / camera mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`. The camera
//! is parked mid-battlefield purely to give a hypothetical pan room.

use core::time::Duration;

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    ecs::schedule::SystemCondition,
    math::Vec2,
    prelude::{Camera2d, IntoScheduleConfigs, Transform, With, resource_exists},
    time::TimeUpdateStrategy,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::{
    GamepadCursorMoved, WorldCamera, cell_to_world, clamp_camera_to_bounds,
    pan_camera_on_gamepad_cursor_edge,
};
use gdtf_battle_sim::{BattleInProgress, Cell, Faction, Level, PlayerFaction};

/// A synthetic window size (logical px) for the edge-band reads.
const WINDOW: Vec2 = Vec2::new(800.0, 600.0);
/// The manual per-update virtual-time delta (the `fx_draw.rs` 250ms idiom) so the pan moves a
/// measurable, deterministic amount each `update()`.
const STEP: Duration = Duration::from_millis(250);
/// The player gang for the battle gate.
const PLAYER_GANG: u8 = 0;

/// Builds the headless edge-pan app: `MinimalPlugins`, the GTW-259 `GamepadCursorMoved`
/// message buffer, the REAL `pan_camera_on_gamepad_cursor_edge` + `clamp_camera_to_bounds`
/// registered exactly as the plugin wires them (battle-gated `Update`, the edge-pan
/// `.before` the clamp so the clamp stays the last writer), a `WorldCamera`, a primary
/// `Window`, and the `BattleInProgress` + `PlayerFaction` gate.
fn edge_pan_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP))
        .add_message::<GamepadCursorMoved>()
        .add_systems(
            Update,
            (
                pan_camera_on_gamepad_cursor_edge,
                clamp_camera_to_bounds.after(pan_camera_on_gamepad_cursor_edge),
            )
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>),
                ),
        );
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));
    // The world camera, parked at the battlefield centre (pan headroom in every direction).
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        Transform::from_translation(battlefield_centre().extend(0.0)),
    ));
    // The primary window so the edge-band size read succeeds.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(WINDOW.x as u32, WINDOW.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));
    app
}

/// Reads the single `WorldCamera`'s translation `xy`.
fn camera_xy(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

/// The battlefield ground-plane world bounds `(min, max)` — the four corner cells of the grid
/// through `cell_to_world` (the SAME projection the clamp uses), so the assertions check a
/// RELATION, not a pinned magnitude.
fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        cell_to_world(Cell::new(0, 0), Level::new(0)),
        cell_to_world(Cell::new(w, 0), Level::new(0)),
        cell_to_world(Cell::new(0, h), Level::new(0)),
        cell_to_world(Cell::new(w, h), Level::new(0)),
    ];
    let mut min = Vec2::new(corners[0].x, corners[0].y);
    let mut max = min;
    for c in corners {
        let p = Vec2::new(c.x, c.y);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

/// The battlefield centre (world space) — pan headroom on every axis.
fn battlefield_centre() -> Vec2 {
    let (min, max) = battlefield_bounds();
    (min + max) * 0.5
}

/// Sends one `GamepadCursorMoved(pos)` message (a TEST-BODY message write — the headless
/// idiom).
fn send_cursor(app: &mut App, pos: Vec2) {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<GamepadCursorMoved>>()
        .write(GamepadCursorMoved::new(pos));
}

/// GTW-271 AC4 — the gamepad edge-pan is VIEWPORT-GATED: with NO map-viewport rect (the
/// headless reality — `camera_system` never runs under `MinimalPlugins`, so
/// `logical_viewport_rect()` is `None`), even a `GamepadCursorMoved` hard at a screen edge
/// produces NO pan (the fail-closed gate). The actual pan-with-a-live-viewport is in-engine QA;
/// the deterministic edge→direction logic is unit-tested on the pure `viewport_edge_dir` helper.
///
/// Pin-discriminating: it sends an edge-band cursor (the position the OLD window-relative
/// edge-pan would have panned toward) and asserts the camera does NOT move — exactly the
/// behaviour the GTW-271 viewport gate introduced (and the "green bar / over-pan" fix). If the
/// system reverted to keying off the whole window it would pan here and fail.
#[test]
fn gamepad_edge_cursor_does_not_pan_without_a_viewport() {
    let mut app = edge_pan_app();
    // Settle one update with no message so the baseline is a clamped, stable position.
    app.update();
    let baseline = camera_xy(&mut app);

    // --- Control: a CENTRED cursor (no edge band) → no move (true regardless of the gate). ---
    send_cursor(&mut app, WINDOW * 0.5);
    app.update();
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "a centred gamepad cursor (no edge band) must NOT move the camera",
    );

    // --- TOP edge (small screen y): the OLD window-relative edge-pan would pan UP here, but the
    // GTW-271 viewport gate fail-closes with no `logical_viewport_rect` (headless), so NO pan. ---
    send_cursor(&mut app, Vec2::new(WINDOW.x * 0.5, 5.0));
    app.update();
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "with no map-viewport rect (headless), an edge-band gamepad cursor must NOT pan — the \
         GTW-271 viewport-inside gate fail-closes",
    );
}

/// AC5 — NO `GamepadCursorMoved` message this update → the camera does not pan (the gamepad is
/// not the active pointer / no edge), proving the edge-pan is message-driven.
#[test]
fn no_message_means_no_pan() {
    let mut app = edge_pan_app();
    app.update();
    let baseline = camera_xy(&mut app);

    // Several updates with NO message — the camera must not drift.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        camera_xy(&mut app),
        baseline,
        "with no GamepadCursorMoved message the camera must not pan",
    );
}
