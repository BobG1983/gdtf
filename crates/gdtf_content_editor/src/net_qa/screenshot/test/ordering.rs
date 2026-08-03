use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    camera::RenderTarget,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, PrimaryWindow, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::{command::CommandOutcome, ids::ShotName, message::QaResponse};
use gdtf_screenshot::{PollCap, SettleFrames};

use super::{
    super::{
        config::{EditorShotPollBudget, EditorShotSettle, EditorShotSource},
        path::{EditorQaShotDir, EditorShotSequence},
        payload::EditorScreenshotPayload,
        pump::{EditorInFlightShots, drive_editor_screenshots},
    },
    support::{TEST_POLL_BUDGET, enqueue, spawned_captures},
};
use crate::net_qa::{present::EditorCapturePresentPlugin, schedule::EditorNetQaSystems};

const HARNESS_SCALE_FACTOR: f32 = 1.25;

fn present_and_pump_app(dir: PathBuf) -> App {
    let mut window = Window::default();
    window.resolution.set_scale_factor(HARNESS_SCALE_FACTOR);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: Some(window),
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.init_resource::<WindowToEguiContextMap>();
    app.init_resource::<PendingQueue<EditorScreenshotPayload>>();
    app.init_resource::<EditorInFlightShots>();
    app.init_resource::<EditorShotSequence>();
    app.init_resource::<EditorShotSource>();
    app.insert_resource(EditorQaShotDir::new(dir));
    app.insert_resource(EditorShotSettle::new(SettleFrames::new(0)));
    app.insert_resource(EditorShotPollBudget::new(PollCap::new(TEST_POLL_BUDGET)));
    app.add_plugins(EditorCapturePresentPlugin);
    app.add_systems(
        Update,
        drive_editor_screenshots.in_set(EditorNetQaSystems::Gather),
    );
    app
}

fn egui_camera_target(app: &mut App) -> Option<RenderTarget> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&RenderTarget, With<PrimaryEguiContext>>();
    let targets: Vec<RenderTarget> = cameras.iter(world).cloned().collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

fn record_egui_window_mapping(app: &mut App, camera: Entity) {
    let mut windows = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    let Some(window) = windows.iter(app.world()).next() else {
        return;
    };
    let mut map = app.world_mut().resource_mut::<WindowToEguiContextMap>();
    map.context_to_window.insert(camera, window);
    map.window_to_contexts
        .entry(window)
        .or_default()
        .insert(camera);
}

#[test]
fn a_shot_settling_on_the_retarget_frame_is_captured_not_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = present_and_pump_app(tmp.path().to_path_buf());
    let camera = app.world_mut().spawn((Camera2d, PrimaryEguiContext)).id();

    app.update();
    let before = egui_camera_target(&mut app);
    assert!(
        matches!(before, Some(RenderTarget::Window(_))),
        "test setup: the camera must still be window-targeted before the input-map entry exists, \
         so the retarget frame is still ahead of us; it aims at {before:?}",
    );
    let source = app.world().get_resource::<EditorShotSource>();
    assert!(
        matches!(source, Some(EditorShotSource::Offscreen(_))),
        "test setup: the present path must have installed the offscreen capture source by now, \
         or the consistency check has nothing to check; the source is {source:?}",
    );

    let reply_rx = enqueue(&mut app, ShotName::new("retarget_frame".to_owned()));
    app.update();
    let claimed = egui_camera_target(&mut app);
    assert!(
        matches!(claimed, Some(RenderTarget::Window(_))),
        "test setup: the shot must be claimed while the camera is still window-targeted; it aims \
         at {claimed:?}",
    );
    assert!(
        spawned_captures(&mut app) == 0,
        "test setup: the capture must not have spawned yet — its consistency check belongs on the \
         NEXT frame, the retarget frame",
    );

    record_egui_window_mapping(&mut app, camera);
    app.update();

    let aimed = egui_camera_target(&mut app);
    assert!(
        matches!(aimed, Some(RenderTarget::Image(_))),
        "test setup: the retarget must have fired on this frame — that is the frame under test; \
         the camera aims at {aimed:?}",
    );
    let reply = reply_rx.try_recv().ok();
    assert!(
        !matches!(
            reply,
            Some(QaResponse::Outcome(CommandOutcome::Unavailable { .. }))
        ),
        "a shot whose settle expired on the retarget frame must NOT be refused: the pump's target \
         read runs after the retarget's write, so it sees the image the camera was just aimed at. \
         It replied {reply:?}",
    );
    assert!(
        reply.is_none() && spawned_captures(&mut app) > 0,
        "the capture must have SPAWNED on the retarget frame rather than answering anything; \
         reply {reply:?}",
    );
}
