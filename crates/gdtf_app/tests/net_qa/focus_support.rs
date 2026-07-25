//! The two Options-resting fixtures the GTW-802 focus suites drive, plus the shared
//! read helpers.
//!
//! Both fixtures rest the REAL Options screen (its real `spawn_options_screen`, its real
//! `add_edges` navigation chain, its real observers) with the REAL `net_qa` router wired to
//! an injected inbox, so every assertion goes through the production path:
//!
//! - [`options_app_with_net_qa`] is the `MinimalPlugins` tier — enough for the ROUTER path
//!   (enumeration, focus movement through the real `apply_navigation`, and the fail-closed
//!   token rejection);
//! - [`ui_options_app_with_net_qa`] is the `DefaultPlugins` tier, needed for ACTIVATION:
//!   under `MinimalPlugins` there is no `InputDispatchPlugin` and no `CheckboxPlugin`, so
//!   the keypress → `FocusedInput` → checkbox half cannot exist there at all. It also
//!   spawns a `Window` + `PrimaryWindow`, which is load-bearing: Bevy's
//!   `dispatch_focused_input` only dispatches when a primary window exists.

use std::sync::mpsc;

use bevy::{
    app::App,
    prelude::Text,
    state::state::{NextState, State},
    window::{PrimaryWindow, Window},
};
use gdtf_app::test_support::{
    AppState, IncomingRequest, NetQaPlugin, RunningState, SoundValueLabel,
};
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    view::FocusView,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

use crate::inject_support::send;

/// How many updates to allow for a transition to settle.
const BUDGET: u32 = 32;

/// Reads the current [`RunningState`], if the sub-state is active.
pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives an already-built app (with its theme seeded) from the menu into the Options
/// screen — the player's Options click, stood in for by the same `NextState` the menu
/// button requests.
fn rest_at_options(app: &mut App) {
    let _ = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    let _ = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Options),
        BUDGET,
    );
}

/// A `MinimalPlugins` app resting on the REAL Options screen with the REAL `net_qa` router
/// wired to an injected inbox. Returns the app and the sender the test pushes requests on
/// (exactly as the listener thread would).
pub(crate) fn options_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    let (tx, rx) = mpsc::channel();
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.add_plugins(NetQaPlugin::with_channels(rx));
    rest_at_options(&mut app);
    (app, tx)
}

/// A `DefaultPlugins` (no GPU, no winit) app resting on the REAL Options screen with the
/// REAL `net_qa` router — the tier the ACTIVATION suite needs, because only `DefaultPlugins`
/// carries `InputPlugin`'s `keyboard_input_system`, `InputDispatchPlugin`'s
/// `dispatch_focused_input`, and `UiWidgetsPlugins`' `CheckboxPlugin`.
///
/// The spawned `Window` + `PrimaryWindow` is load-bearing, not decoration: an emitted
/// `KeyboardInput` carries a window entity, and `dispatch_focused_input` returns early
/// without a primary window — so no `FocusedInput` would ever reach the checkbox.
pub(crate) fn ui_options_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    let (tx, rx) = mpsc::channel();
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_plugins(NetQaPlugin::with_channels(rx));
    rest_at_options(&mut app);
    (app, tx)
}

/// Send `GetAppFlow`, drive one frame, and read the folded focus view off the reply — the
/// enumeration a client actually polls.
pub(crate) fn read_focus(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Option<FocusView> {
    let reply = send(tx, QaRequest::GetAppFlow);
    app.update();
    let Ok(QaResponse::AppFlow(view)) = reply.try_recv() else {
        unreachable!("GetAppFlow must answer with an AppFlow snapshot");
    };
    view.focus
}

/// Reads the text of the single [`SoundValueLabel`] — the on-screen readout the screen's
/// own `sync_sound_value_label` writes from `GameSettings`, so a flip here proves the whole
/// activation chain landed.
pub(crate) fn sound_value_text(app: &mut App) -> Option<String> {
    let mut query = app
        .world_mut()
        .query_filtered::<&Text, bevy::ecs::prelude::With<SoundValueLabel>>();
    let found: Vec<String> = query
        .iter(app.world())
        .map(|text| text.as_str().to_owned())
        .collect();
    match found.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}
