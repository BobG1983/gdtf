use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::gpu_readback::Readback,
    ui::{ComputedNode, ComputedUiTargetCamera, Interaction, UiGlobalTransform},
    window::PrimaryWindow,
};
use gdtf_app::test_support::{EndTurnButton, NetQaPlugin};
use gdtf_qa_protocol::ports::NetQaPort;
use gdtf_screenshot::{
    CapturePipelinePlugin, CaptureQueue, PollCap, SettleFrames, ShotDir, ShotStem,
};
use tempfile::TempDir;

use super::real_layout_harness::real_layout_battle_running_app;

const TEST_SETTLE: u32 = 2;

// Wide enough that the cursor can be driven off the button and back while a capture is in flight.
const TEST_POLL_BUDGET: u32 = 12;

const CAPTURE_FRAME_BUDGET: u32 = 64;

const HOVER_FRAMES: u32 = 4;

const PROBE_FRAMES: u32 = 2;

/// A logical pixel inside the window and off every action-bar button.
const OFF_BUTTON_PX: Vec2 = Vec2::new(1.0, 1.0);

/// The real-layout battle app with the QA present path and a drivable capture queue.
fn battle_app_with_the_qa_capture_path() -> (App, TempDir) {
    let app_opt = real_layout_battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real-layout harness must reach BattleScapeState::BattleRunning with its font loaded",
    );
    let Some(mut app) = app_opt else {
        unreachable!("the assertion above leaves the app present")
    };
    let Ok((plugin, _port)) = NetQaPlugin::listening(NetQaPort::new(0)) else {
        unreachable!("the OS must hand out a loopback port when asked for port 0")
    };
    app.add_plugins(plugin);
    app.add_plugins(CapturePipelinePlugin::<()>::new());
    let Ok(tmp) = TempDir::new() else {
        unreachable!("a temp directory is available")
    };
    app.insert_resource(ShotDir::new(tmp.path().to_path_buf()));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
    app.update();
    (app, tmp)
}

/// The one laid-out End Turn button.
fn end_turn_button(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<EndTurnButton>>();
    let found: Vec<Entity> = query.iter(world).collect();
    let [button] = found.as_slice() else {
        unreachable!("exactly one End Turn button must exist in a running battle: {found:?}")
    };
    *button
}

/// The button's centre in logical pixels, which is what `Window::set_cursor_position` takes.
fn logical_centre_of(app: &App, button: Entity) -> Option<Vec2> {
    let node = app.world().get::<ComputedNode>(button)?;
    let transform = app.world().get::<UiGlobalTransform>(button)?;
    Some(transform.translation * node.inverse_scale_factor())
}

fn physical_size_of(app: &App, button: Entity) -> Option<Vec2> {
    app.world()
        .get::<ComputedNode>(button)
        .map(ComputedNode::size)
}

fn cursor_at(app: &mut App, position: Option<Vec2>) {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    for mut window in windows.iter_mut(world) {
        window.set_cursor_position(position);
    }
}

fn physical_cursor(app: &mut App) -> Option<Vec2> {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    windows
        .iter(world)
        .next()
        .and_then(Window::physical_cursor_position)
}

fn interaction_of(app: &App, button: Entity) -> Option<Interaction> {
    app.world().get::<Interaction>(button).copied()
}

/// What the camera this button resolves to renders into, if it resolves to one at all.
fn ui_camera_target(app: &App, button: Entity) -> Option<RenderTarget> {
    let camera = app
        .world()
        .get::<ComputedUiTargetCamera>(button)
        .and_then(ComputedUiTargetCamera::get)?;
    app.world().get::<RenderTarget>(camera).cloned()
}

/// The same reading, spelled for a failure message.
fn ui_render_target(app: &App, button: Entity) -> String {
    match ui_camera_target(app, button) {
        Some(target) => format!("{target:?}"),
        None => "nothing — the button resolved to no camera, or to one carrying no RenderTarget"
            .to_owned(),
    }
}

// ui_focus_system writes no Interaction at all for a node that resolves to no camera.
fn assert_resolves_to_a_window_camera(app: &App, button: Entity, when: &str) {
    let target = ui_camera_target(app, button);
    assert!(
        matches!(target, Some(RenderTarget::Window(_))),
        "{when} the hovered button must resolve to a camera rendering to a window; it found \
         {target:?}. bevy_ui builds a cursor position only for window-targeted cameras, and a \
         node resolving to none is skipped outright — either way nothing writes Interaction and \
         the button keeps whatever value it already held",
    );
}

/// Park the cursor on the button and let `ui_focus_system` see it.
fn hover_the_button(app: &mut App, button: Entity) {
    let Some(centre) = logical_centre_of(app, button) else {
        unreachable!("the End Turn button must carry a ComputedNode and a UiGlobalTransform")
    };
    for _ in 0..HOVER_FRAMES {
        cursor_at(app, Some(centre));
        app.update();
    }
}

fn a_capture_is_in_flight(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut readbacks = world.query::<&Readback>();
    readbacks.iter(world).next().is_some()
}

fn enqueue_capture(app: &mut App, stem: &str) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new(stem)), ());
}

/// Run frames until the pump has a readback in flight.
fn drive_until_in_flight(app: &mut App) -> bool {
    for _ in 0..CAPTURE_FRAME_BUDGET {
        app.update();
        if a_capture_is_in_flight(app) {
            return true;
        }
    }
    false
}

/// Park the cursor and run a couple of frames so `ui_focus_system` acts on it.
fn settle_cursor_at(app: &mut App, position: Vec2) {
    for _ in 0..PROBE_FRAMES {
        cursor_at(app, Some(position));
        app.update();
    }
}

#[test]
fn a_pointer_resting_on_a_button_reaches_hovered() {
    let (mut app, _tmp) = battle_app_with_the_qa_capture_path();
    let button = end_turn_button(&mut app);

    let size = physical_size_of(&app, button);
    assert!(
        size.is_some_and(|size| size.x > 0.0 && size.y > 0.0),
        "the End Turn button must have a laid-out, non-zero ComputedNode size before a hover can \
         hit it; it is {size:?}",
    );

    hover_the_button(&mut app, button);

    let found = interaction_of(&app, button);
    assert_eq!(
        found,
        Some(Interaction::Hovered),
        "a pointer resting on the End Turn button must reach Interaction::Hovered; it is {found:?} \
         and the camera it resolves to renders into {}. ui_focus_system only builds a cursor \
         position for cameras rendering to a window, so an image target here is the cause and a \
         Window target means the hit test missed",
        ui_render_target(&app, button),
    );
}

#[test]
fn a_capture_leaves_the_hovered_button_hovered() {
    let (mut app, _tmp) = battle_app_with_the_qa_capture_path();
    let button = end_turn_button(&mut app);
    hover_the_button(&mut app, button);
    assert_eq!(
        interaction_of(&app, button),
        Some(Interaction::Hovered),
        "test setup: the button must be Hovered before the capture; the camera it resolves to \
         renders into {}",
        ui_render_target(&app, button),
    );
    let before = physical_cursor(&mut app);

    enqueue_capture(&mut app, "hover_survives");

    let mut started = false;
    let mut idle = false;
    for _ in 0..CAPTURE_FRAME_BUDGET {
        app.update();
        if a_capture_is_in_flight(&mut app) {
            started = true;
            assert_resolves_to_a_window_camera(&app, button, "while a capture is in flight");
        }
        if app.world().resource::<CaptureQueue<()>>().is_idle() {
            idle = true;
            break;
        }
    }
    assert!(
        started,
        "test setup: the capture must actually start — no entity carried a Readback in \
         {CAPTURE_FRAME_BUDGET} frames, so this case would say nothing about hover surviving one",
    );
    assert!(
        idle,
        "test setup: the capture queue must reach idle inside {CAPTURE_FRAME_BUDGET} frames",
    );

    assert_eq!(
        physical_cursor(&mut app),
        before,
        "test setup: the capture must not move the cursor, or the hover assertion below proves \
         nothing",
    );
    assert_resolves_to_a_window_camera(&app, button, "after a capture");
    let found = interaction_of(&app, button);
    assert_eq!(
        found,
        Some(Interaction::Hovered),
        "taking a capture must leave the hovered button Hovered; it is {found:?} and the camera \
         it resolves to renders into {}",
        ui_render_target(&app, button),
    );
}

#[test]
fn interaction_is_still_written_while_a_capture_is_in_flight() {
    let (mut app, _tmp) = battle_app_with_the_qa_capture_path();
    let button = end_turn_button(&mut app);
    hover_the_button(&mut app, button);
    let Some(centre) = logical_centre_of(&app, button) else {
        unreachable!("the End Turn button must carry a ComputedNode and a UiGlobalTransform")
    };

    enqueue_capture(&mut app, "hover_is_live");
    assert!(
        drive_until_in_flight(&mut app),
        "test setup: the capture must reach a readback inside {CAPTURE_FRAME_BUDGET} frames",
    );

    settle_cursor_at(&mut app, OFF_BUTTON_PX);
    let off_button = interaction_of(&app, button);
    let still_in_flight = a_capture_is_in_flight(&mut app);
    settle_cursor_at(&mut app, centre);
    let back_on_button = interaction_of(&app, button);

    assert!(
        still_in_flight,
        "test setup: the capture must still be in flight while the cursor is off the button, or \
         this case says nothing about a capture frame; widen TEST_POLL_BUDGET, now \
         {TEST_POLL_BUDGET}",
    );
    assert_eq!(
        off_button,
        Some(Interaction::None),
        "a capture frame must leave ui_focus_system writing Interaction: with the cursor moved \
         off the button mid-capture it must read None, not {off_button:?}. A stale Hovered here \
         means nothing wrote the component at all — the camera it resolves to renders into {}",
        ui_render_target(&app, button),
    );
    assert_eq!(
        back_on_button,
        Some(Interaction::Hovered),
        "moving the cursor back onto the button mid-capture must reach Hovered again, not \
         {back_on_button:?}",
    );
}
