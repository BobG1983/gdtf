use bevy::prelude::*;

use super::super::{
    dwell::{DwellElapsed, PanEdgeDwellState},
    marker::WorldCamera,
    pan::pan_camera,
    tuning::{DwellDelaySeconds, PanTuning},
};

#[test]
fn pan_camera_keyboard_pans_regardless_of_cursor() {
    use std::time::Duration;

    use bevy::{
        time::TimeUpdateStrategy,
        window::{PrimaryWindow, Window},
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::KeyW);
    app.insert_resource(keys);
    app.add_systems(Update, pan_camera);

    app.world_mut().spawn((
        WorldCamera,
        Camera::default(),
        Transform::from_translation(Vec3::ZERO),
    ));
    app.world_mut().spawn((Window::default(), PrimaryWindow));

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
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.init_resource::<PanEdgeDwellState>();
    app.insert_resource(PanTuning {
        dwell_delay_seconds: DwellDelaySeconds::new(0.3),
        ..PanTuning::default()
    });
    app.add_systems(Update, pan_camera);

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

    let mut window = Window::default();
    window.set_cursor_position(Some(Vec2::new(5.0, 300.0)));
    app.world_mut().spawn((window, PrimaryWindow));

    let camera_x = |app: &mut App| {
        app.world()
            .get::<Transform>(camera_entity)
            .map_or(0.0, |t| t.translation.x)
    };

    app.update();
    app.update();
    app.update();
    let below_dwell = camera_x(&mut app);
    assert!(
        below_dwell.abs() < f32::EPSILON,
        "after < dwell-delay linger in the edge band the camera must NOT pan, got x={below_dwell}",
    );

    app.update();
    app.update();
    let after_dwell = camera_x(&mut app);
    assert!(
        after_dwell < below_dwell - f32::EPSILON,
        "once the cursor has dwelt >= the dwell delay the camera must pan left (-X): \
         {below_dwell} -> {after_dwell}",
    );

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
