//! GTW-259 (AC5): the GAMEPAD-cursor edge-pan through the REAL registered
//! `pan_camera_on_gamepad_cursor_edge`, headless.
//!
//! Sends a `GamepadCursorMoved(pos near a screen edge)` and drives one `update()`: the
//! `WorldCamera` pans toward that edge (REUSING the GTW-250 `mouse_edge_dir` / `pan_velocity`
//! helpers) and the clamp (which runs after) keeps it inside the battlefield bounds; a centred
//! position pans nothing. This proves the SETTABLE-message edge-pan logic — the input crate
//! emits this same message when the gamepad is the active pointer, and the raw gamepad reads
//! are TBD-Bevy-harness / in-engine QA (`verification.md`, the contract's honesty clause).
//!
//! The systems are exercised on their REAL registration shape (battle-gated `Update`,
//! `.before(clamp_camera_to_bounds)`), not a copy. Every `app.world_mut()` / camera mutation
//! is in a TEST BODY — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No
//! function here takes `&mut World`/`&World`. The camera is parked mid-battlefield purely to
//! give the pan room (so the clamp is not the boundary case).

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
                .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>)),
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
        .write(GamepadCursorMoved(pos));
}

/// AC5 — a `GamepadCursorMoved` near the TOP edge pans the camera UP (`+Y`), and the clamp
/// keeps it inside the battlefield bounds; a centred position pans nothing.
#[test]
fn gamepad_cursor_at_top_edge_pans_up_within_bounds() {
    let mut app = edge_pan_app();
    // Settle one update with no message so the baseline is a clamped, stable position.
    app.update();
    let baseline = camera_xy(&mut app);

    // --- Control: a CENTRED cursor (no edge band) → no message effect → no move. ---
    send_cursor(&mut app, WINDOW * 0.5);
    app.update();
    let centred = camera_xy(&mut app);
    assert_eq!(
        centred, baseline,
        "a centred gamepad cursor (no edge band) must NOT move the camera",
    );

    // --- TOP edge (small screen y) → pan the camera UP (+Y). ---
    // Screen y near 0 is the TOP band; the helper flips it to camera +Y.
    send_cursor(&mut app, Vec2::new(WINDOW.x * 0.5, 5.0));
    app.update();
    let after_top = camera_xy(&mut app);
    assert!(
        after_top.y > baseline.y,
        "a gamepad cursor at the TOP edge must pan the camera UP (+Y): baseline y {} -> after {}",
        baseline.y,
        after_top.y,
    );

    // The pan stayed within the battlefield bounds — the clamp ran AFTER the pan.
    let (min, max) = battlefield_bounds();
    assert!(
        after_top.y <= max.y + f32::EPSILON && after_top.y >= min.y - f32::EPSILON,
        "after the pan the camera y ({}) must stay within the battlefield y bounds [{}, {}]",
        after_top.y,
        min.y,
        max.y,
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
