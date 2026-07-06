//! GTW-250 (AC5): the KEYBOARD pan path through the REAL registered `pan_camera`
//! system, in a live battle.
//!
//! Drives the full GDTF state machine into a live `GameState::BattleScape` (a valid
//! two-ganger situation, so `setup_battle` inserts the sim's `BattleInProgress` +
//! `PlayerFaction` — the battle gate the camera systems run under) and then exercises the
//! presenter's `pan_camera` exactly as the app wires it (in `Update`, battle-gated,
//! `.before(clamp_camera_to_bounds)`). Pressing `KeyCode::KeyW` (set via the
//! `ButtonInput<KeyCode>` resource, which `InputPlugin` provides under `register_headless`)
//! moves the `WorldCamera` `translation.y` UP and the clamp keeps it inside the battlefield;
//! pressing nothing leaves the camera put.
//!
//! The mouse-edge + gamepad paths are covered by their pure helper unit tests in
//! `gdtf_battle_presenter::world_camera` + in-engine QA (real window cursor / real gamepad
//! axes are TBD-Bevy-harness per `verification.md`).
//!
//! All `app.world_mut()` / camera mutation is in the TEST BODY — the accepted headless
//! idiom (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.
//! The camera is repositioned mid-battlefield in the test body purely to give the pan room
//! to move (so the clamp is not the boundary case) — the SYSTEM under test is the real
//! registered `pan_camera`, not a copy.

use bevy::{
    ecs::prelude::With,
    input::{ButtonInput, keyboard::KeyCode},
    math::Vec2,
    transform::components::Transform,
};
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    battle::{BattleInProgress, PlayerFaction},
    metric::{Cell, Level},
};
use gdtf_test_utils::BattleAppBuilder;

/// How many updates to hold a pan key — enough for the per-update virtual-time delta
/// (`FixedTimesteps(1)`, ~1/64 s) to accumulate a clearly measurable, non-clamped move.
const PAN_UPDATES: u32 = 12;

/// Builds the headless app already driven to a live battle via the shared
/// [`BattleAppBuilder`] (the default `fixtures::two_ganger` situation, seeded with the
/// canonical `Load` resources + test registries). It rests at
/// `BattleScapeState::BattleRunning`, which is PAST `Generation`, so the successful
/// `setup_battle` has inserted [`BattleInProgress`] + [`PlayerFaction`] — the gate the
/// camera systems run under. Returns `None` if the shared drive does not reach the live
/// battle (the caller asserts the `Some`).
fn live_battle_app() -> Option<bevy::app::App> {
    BattleAppBuilder::new().build()
}

/// Reads the single `WorldCamera`'s translation `xy`.
fn camera_xy(app: &mut bevy::app::App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

/// Sets the single `WorldCamera`'s translation `xy` (a TEST-BODY world mutation), giving the
/// pan room to move so the clamp is not the boundary case.
fn set_camera_xy(app: &mut bevy::app::App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}

/// The battlefield ground-plane world bounds `(min, max)` — the four corner cells of the
/// `GRID_WIDTH x GRID_HEIGHT` ground extent through `cell_to_world` (the SAME projection the
/// clamp uses). Kept local so the assertions check a RELATION, not a pinned magnitude.
fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::occupancy::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::occupancy::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        gdtf_battle_presenter::cell_to_world(Cell::new(0, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, 0), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(0, h), Level::new(0)),
        gdtf_battle_presenter::cell_to_world(Cell::new(w, h), Level::new(0)),
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

/// The battlefield centre (world space) — a position with pan headroom on every axis.
fn battlefield_centre() -> Vec2 {
    let (min, max) = battlefield_bounds();
    (min + max) * 0.5
}

/// Holds `key` pressed across `PAN_UPDATES` updates so the per-frame pan accumulates a
/// measurable move; the `ButtonInput<KeyCode>` is re-pressed each frame because
/// `InputPlugin`'s `clear` runs every update.
fn hold_key_for_pan(app: &mut bevy::app::App, key_code: KeyCode) {
    for _ in 0..PAN_UPDATES {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key_code);
        app.update();
    }
}

/// AC5 — pressing `KeyCode::KeyW` in a live battle pans the `WorldCamera` UP (`+Y`) through
/// the REAL registered `pan_camera`, and the clamp (which runs after) keeps it inside the
/// battlefield; pressing nothing leaves the camera put.
#[test]
fn keyboard_w_pans_camera_up_within_bounds() {
    let app_opt = live_battle_app();
    assert!(
        app_opt.is_some(),
        "the shared BattleAppBuilder drive should reach a LIVE battle (rested at \
         BattleScapeState::BattleRunning, past Generation, so BattleInProgress + PlayerFaction \
         are present)",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // The system must be battle-gated AND actually running — both witnesses present.
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some()
            && app.world().get_resource::<PlayerFaction>().is_some(),
        "precondition: the live battle's BattleInProgress + PlayerFaction gate the camera systems",
    );

    // Park the camera at the battlefield centre so the pan has room in every direction (the
    // clamp is not the boundary case for a small up-pan).
    let centre = battlefield_centre();
    set_camera_xy(&mut app, centre);
    // Settle the clamp once so the baseline is a clamped, stable position.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let baseline = camera_xy(&mut app);

    // --- Control: no key pressed → the camera does not move. ---
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    let no_press = camera_xy(&mut app);
    assert_eq!(
        no_press, baseline,
        "with no pan key pressed, the camera must NOT move",
    );

    // --- W held → the camera pans UP (+Y). ---
    hold_key_for_pan(&mut app, KeyCode::KeyW);
    let after_w = camera_xy(&mut app);
    assert!(
        after_w.y > baseline.y,
        "holding W must pan the WorldCamera UP (+Y): baseline y {} -> after {}",
        baseline.y,
        after_w.y,
    );

    // The pan stayed within the battlefield bounds — the clamp ran AFTER the pan (it is the
    // last writer). The half-viewport is the headless default unit rect, so the camera centre
    // must sit within [min, max] (a far tighter bound than [min+half, max-half]).
    let (min, max) = battlefield_bounds();
    assert!(
        after_w.y <= max.y + f32::EPSILON && after_w.y >= min.y - f32::EPSILON,
        "after the pan the camera y ({}) must stay within the battlefield y bounds [{}, {}] — the \
         clamp ran after the pan",
        after_w.y,
        min.y,
        max.y,
    );
    assert!(
        after_w.x <= max.x + f32::EPSILON && after_w.x >= min.x - f32::EPSILON,
        "the camera x must stay within the battlefield x bounds",
    );
}
