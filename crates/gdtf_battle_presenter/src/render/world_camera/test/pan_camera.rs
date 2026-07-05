//! Tests of the registered `pan_camera` system driven headlessly — the keyboard
//! source and the mouse-edge dwell gate, end-to-end.

use bevy::prelude::*;

use super::super::{
    dwell::{DwellElapsed, PanEdgeDwellState},
    marker::WorldCamera,
    pan::pan_camera,
    tuning::{DwellDelaySeconds, PanTuning},
};

/// GTW-271 — `pan_camera`'s KEYBOARD source is NOT cursor-bound: it pans regardless of where
/// the cursor is (the GTW-262 `Interaction`-over-UI early-return that wrongly froze the
/// keyboard pan is GONE). This pins the refactor-preserved half of the pan system through the
/// real registered system, with the camera now carrying a `Camera` component (the AC4 query
/// reads `&Camera` for `logical_viewport_rect`).
///
/// Pin-discriminating: W is pressed and the keyboard pan must move the camera +Y across an
/// update — if the removed `pointer_over_ui` gate were still suppressing all sources, or if
/// the keyboard branch regressed, the camera would not move.
#[test]
fn pan_camera_keyboard_pans_regardless_of_cursor() {
    use std::time::Duration;

    use bevy::{
        time::TimeUpdateStrategy,
        window::{PrimaryWindow, Window},
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // A controlled non-zero per-update delta so the pan is measurable (the virtual clock
    // reports 0 on the first update and clamps each step to 250ms — ample at PAN_SPEED).
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyW);
    app.insert_resource(keys);
    app.add_systems(Update, pan_camera);

    // The camera carries a `Camera` component (the AC4 query reads `&Camera`); headlessly
    // `logical_viewport_rect()` is `None`, so the mouse-edge source contributes nothing and
    // only the (cursor-independent) keyboard source drives the pan.
    app.world_mut().spawn((
        WorldCamera,
        Camera::default(),
        Transform::from_translation(Vec3::ZERO),
    ));
    app.world_mut().spawn((Window::default(), PrimaryWindow));

    // Warm up once (the first-update zero delta), then measure the keyboard pan.
    app.update();
    let before = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<WorldCamera>>();
        q.iter(app.world()).next().map_or(0.0, |t| t.translation.y)
    };
    app.update();
    let after = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<WorldCamera>>();
        q.iter(app.world()).next().map_or(0.0, |t| t.translation.y)
    };
    assert!(
        after > before,
        "with W pressed the keyboard pan must move the camera up (+Y) regardless of the \
         cursor: {before} -> {after}",
    );
}

// -----------------------------------------------------------------------------
// GTW-299 — the edge-pan DWELL gate: the camera does not start a MOUSE-edge pan until the
// cursor has rested in the edge band for >= the dwell delay.
// -----------------------------------------------------------------------------

/// GTW-299 AC1/AC6 — the REAL `pan_camera` mouse-edge path, driven headlessly over multiple
/// updates: a cursor held in the LEFT edge band for LESS than the dwell delay must NOT move the
/// camera; held PAST the dwell delay it pans; then moving the cursor OUT of the band resets the
/// accumulator so the pan stops.
///
/// Mirrors `pan_camera_keyboard_pans_regardless_of_cursor` (the real registered system over
/// `app.update()`s with a manual clock), but spawns a `Camera` with a NON-`None` viewport rect (via
/// `computed.target_info`) and a `Window` with a cursor IN the edge band, so the mouse-edge source
/// is live. The `PanEdgeDwellState` resource is inserted (as the registration does) so the dwell
/// gate exercises the real accumulate/reset path, and `PanTuning` pins the dwell delay so the
/// frame budget is deterministic.
///
/// Pin-discriminating: with the dwell gate removed the FIRST in-band frame would already pan, so
/// the "below-dwell must not move" assert is the discriminator; the reset assert pins the
/// leave-the-band branch.
#[test]
fn pan_camera_mouse_edge_waits_for_the_dwell_delay() {
    use std::time::Duration;

    use bevy::{
        camera::{ComputedCameraValues, RenderTargetInfo},
        time::TimeUpdateStrategy,
        window::{PrimaryWindow, Window},
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // 100 ms per update — a deterministic step against a 300 ms dwell (3 in-band updates reach it).
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    // No keyboard input — only the mouse-edge source may move the camera.
    app.insert_resource(ButtonInput::<KeyCode>::default());
    // The dwell gate's state + a pinned 300 ms dwell delay (shipped speed / edge band defaults).
    app.init_resource::<PanEdgeDwellState>();
    app.insert_resource(PanTuning {
        dwell_delay_seconds: DwellDelaySeconds::new(0.3),
        ..PanTuning::default()
    });
    app.add_systems(Update, pan_camera);

    // A camera with a NON-None viewport rect: an 800x600 target at scale 1.0 makes
    // `logical_viewport_rect()` => Rect(0,0 .. 800,600), so the mouse edge bands against it.
    let camera = Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor:  1.0,
            }),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    };
    let camera_entity = app
        .world_mut()
        .spawn((WorldCamera, camera, Transform::from_translation(Vec3::ZERO)))
        .id();

    // A window with the cursor parked in the LEFT edge band (x = 5 < 24 px band).
    let mut window = Window::default();
    window.set_cursor_position(Some(Vec2::new(5.0, 300.0)));
    app.world_mut().spawn((window, PrimaryWindow));

    let camera_x = |app: &mut App| {
        app.world()
            .get::<Transform>(camera_entity)
            .map_or(0.0, |t| t.translation.x)
    };

    // The virtual clock reports a ZERO delta on the FIRST update, then 100 ms each after — so the
    // first update warms up (accumulates nothing) and each later update adds 0.1 s of in-band
    // dwell. Three updates => one warm-up + two 100 ms steps => 0.2 s accumulated, still BELOW the
    // 0.3 s threshold: NO pan yet.
    app.update();
    app.update();
    app.update();
    let below_dwell = camera_x(&mut app);
    assert!(
        below_dwell.abs() < f32::EPSILON,
        "after < dwell-delay linger in the edge band the camera must NOT pan, got x={below_dwell}",
    );

    // Two MORE 100 ms updates push the accumulated dwell to ~0.4 s (comfortably >= 0.3 s): the
    // gate opens and the cursor's left-edge band pans the camera LEFT (-X).
    app.update();
    app.update();
    let after_dwell = camera_x(&mut app);
    assert!(
        after_dwell < below_dwell - f32::EPSILON,
        "once the cursor has dwelt >= the dwell delay the camera must pan left (-X): \
         {below_dwell} -> {after_dwell}",
    );

    // Move the cursor OUT of the edge band (centre): the accumulator resets, the pan stops, and
    // the camera holds its position across the next update.
    {
        let mut windows = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        if let Some(mut window) = windows.iter_mut(app.world_mut()).next() {
            window.set_cursor_position(Some(Vec2::new(400.0, 300.0)));
        }
    }
    app.update();
    let after_reset = camera_x(&mut app);
    assert!(
        (after_reset - after_dwell).abs() < f32::EPSILON,
        "with the cursor moved out of the edge band the pan must stop (no further movement): \
         {after_dwell} -> {after_reset}",
    );
    let dwell_state = app.world().resource::<PanEdgeDwellState>();
    assert_eq!(
        dwell_state.mouse,
        DwellElapsed::ZERO,
        "leaving the edge band must reset the mouse dwell accumulator to zero",
    );
}
