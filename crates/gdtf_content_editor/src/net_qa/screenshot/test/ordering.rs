//! The pump's target read is ordered AFTER the present path's retarget write (GTW-922).
//!
//! The pre-spawn consistency check ([`super::super::aim`]) reads the egui camera's
//! [`RenderTarget`], which the present path's `retarget_editor_camera_to_offscreen` writes
//! through [`Commands`]. Two systems that read and write the same component with no explicit
//! ordering run in a nondeterministic order (bevy-traps #3), and Bevy only guarantees one
//! system's deferred commands are applied before another runs when there is an explicit ordering
//! between them. Unordered, a screenshot that settles on the very frame the retarget fires is
//! either captured or refused `TargetNotRendered` depending on how the executor scheduled the
//! two — a one-frame-off flake, and a loud refusal of a capture that would have been correct.
//!
//! `EditorNetQaSystems::Present` before `EditorNetQaSystems::Gather` (configured by
//! `EditorCapturePresentPlugin`) is that ordering. The test below drives the frame the ordering
//! decides: the shot is claimed on a frame where the camera is still window-targeted, and its
//! settle expires on the frame the retarget aims it at the offscreen image.
//!
//! Both schedules are co-resident here — the REAL present plugin and the REAL pump in one app,
//! registered the way `NetQaEditorPlugin::serve` registers them. The rest of the pump's tests
//! (`support::pump_app`) run the pump alone on `MinimalPlugins`, which is why none of them could
//! see this.

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
use gdtf_qa_protocol::{
    envelope::{QaResponse, ScreenshotResult},
    ids::ShotName,
};
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

/// The scale factor this app's window reports. Deliberately not `1.0`, and distinct from every
/// other scale-factor fixture in the crate — `1.5` in `present/test/harness.rs`, `2.5` in
/// `test/aim.rs`, `3.0` in `test/source.rs`, `2.0` in `tests/net_qa_editor_present/harness.rs` and
/// `1.75` in `tests/net_qa_editor_screenshot/harness.rs` — so no single hardcoded scale factor can
/// satisfy this test and any of the others (GTW-922 clause 6).
const HARNESS_SCALE_FACTOR: f32 = 1.25;

/// Build an app carrying BOTH the real present path and the real pump, wired the way
/// `NetQaEditorPlugin::serve` wires them: the present plugin's own `Update` chain, and the pump
/// registered in [`EditorNetQaSystems::Gather`].
///
/// A real primary window (no GPU backend, no winit event loop) is required — the present path
/// sizes its offscreen image from the window. The settle window is ZERO so a claimed shot
/// reaches its consistency check on the NEXT frame, which is what lets the test land that check
/// on the retarget frame.
///
/// `Gather`'s production `run_if(resource_exists::<State<EditorState>>)` is left off: it gates
/// WHETHER the band runs, never its order, and this app has no editor state machine.
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
    // With no render backend some render-provided system params cannot validate, and Bevy 0.19
    // routes a failed validation to the global error handler (panicking by default). `warn`
    // restores skip-with-a-log — the shared headless-harness precedent.
    app.set_error_handler(warn);
    // The input map `bevy_egui` owns in production. This app has no `EguiPlugin` (it has no GPU
    // to give it), and the entry is inserted BY HAND MID-TEST on purpose: that is what pins the
    // retarget to a known frame. That the REAL `bevy_egui` records this entry for the editor's
    // real camera is proven separately, on the real editor app, by
    // `tests/net_qa_editor_present`.
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

/// The render target of the single camera holding the primary egui context, or [`None`] unless
/// there is exactly one.
fn egui_camera_target(app: &mut App) -> Option<RenderTarget> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&RenderTarget, With<PrimaryEguiContext>>();
    let targets: Vec<RenderTarget> = cameras.iter(world).cloned().collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// Record the `bevy_egui` input-map entry `on_egui_context_added_system` records for a
/// window-targeted context — the entry the retarget waits for.
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

/// A shot whose settle window expires on the SAME frame the retarget aims the camera at the
/// offscreen image is CAPTURED, not refused.
///
/// The frame is constructed, not waited for: the shot is claimed while the `bevy_egui` input-map
/// entry is still missing (so the retarget cannot fire), the entry is then recorded, and the next
/// frame carries both the retarget's write and the pump's consistency check. With
/// `Present`-before-`Gather` the check reads the target the retarget just installed and the
/// capture spawns. Without that ordering the check can read the stale
/// `RenderTarget::Window` from the same frame and answer `TargetNotRendered` — the flake this
/// pins, and the reason the ordering is explicit rather than inherited from registration order.
#[test]
fn a_shot_settling_on_the_retarget_frame_is_captured_not_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = present_and_pump_app(tmp.path().to_path_buf());
    let camera = app.world_mut().spawn((Camera2d, PrimaryEguiContext)).id();

    // Frame 1: the present path creates the offscreen target and names it as the capture source.
    // The retarget cannot fire — no input-map entry yet.
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

    // Frame 2: the pump CLAIMS the shot (settle 0, so its check runs next frame). The retarget
    // is still held back.
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

    // Frame 3 — the frame the ordering decides: the retarget writes and the pump's consistency
    // check reads, both in this one `Update`.
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
            Some(QaResponse::Screenshot(ScreenshotResult::TargetNotRendered(
                _
            )))
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
