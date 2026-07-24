//! Shared setup for the GTW-819 coexistence suite: build the REAL Load flow with the spike
//! plugin added, drive it into `AppState::Running`, and read the two stacks' observables.
//!
//! # Why the egui click is injected as raw input
//!
//! The headless harness runs with `primary_window: None` (no OS window, no cursor), so
//! `bevy_egui`'s own input systems have no pointer to read — the ONE thing a windowless app
//! cannot supply. [`inject_egui_pointer_click`] therefore writes the pointer events into the
//! context's [`EguiInput`] (the exact buffer `bevy_egui`'s `write_egui_input_system` fills from
//! window events), during `Update`, so `run_egui_context_pass_loop_system` takes them in
//! `PostUpdate` the way it takes real ones. Everything downstream of that is the REAL path:
//! egui hit-tests the button rect itself and the production `ui.button(..).clicked()` branch is
//! what fires. Nothing about the spike's systems is stubbed.

use bevy::{
    app::{App, Update},
    ecs::{entity::Entity, resource::Resource},
    prelude::{Commands, Component, Query, Res, With},
    state::state::State,
    ui::ComputedNode,
};
use bevy_egui::{EguiInput, PrimaryEguiContext, egui};
use gdtf_app::test_support::{
    AppState, CoexistenceBevyUiButton, EGUI_PANEL_POS, UiCoexistencePlugin, UiStackClicks,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads plus the state descent
/// into `Running` under contention (the `procgen_stepper` suite's precedent).
pub(crate) const BUDGET: u32 = 512;

/// The screen extent handed to egui, in points.
///
/// `bevy_egui` normally derives this from the primary window; there is none here, and an egui
/// context with no screen rect clips every widget away — so the harness supplies one big enough
/// to contain the spike's panel.
const HEADLESS_SCREEN: egui::Vec2 = egui::vec2(1280.0, 720.0);

/// Requests one synthetic egui pointer click on the spike's egui button.
///
/// A fieldless marker resource: the test inserts it, [`inject_egui_pointer_click`] consumes it
/// on the next `Update`, so the click is delivered on a known frame.
#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct PendingEguiClick;

/// `Update`: deliver a queued pointer press+release to the primary egui context.
///
/// Aims at the interior of the spike's egui button — the button is the panel's first widget, so
/// its top-left is `EGUI_PANEL_POS` (see the production module's doc) and a small inset lands
/// inside its rect.
pub(crate) fn inject_egui_pointer_click(
    pending: Option<Res<PendingEguiClick>>,
    mut contexts: Query<&mut EguiInput, With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    if pending.is_none() {
        return;
    }
    let pos = egui::pos2(EGUI_PANEL_POS.x + 8.0, EGUI_PANEL_POS.y + 8.0);
    let mut delivered = false;
    for mut input in &mut contexts {
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, HEADLESS_SCREEN));
        input.events.push(egui::Event::PointerMoved(pos));
        for pressed in [true, false] {
            input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        delivered = true;
    }
    if delivered {
        commands.remove_resource::<PendingEguiClick>();
    }
}

/// The current [`AppState`], if the state stack is up.
pub(crate) fn app_state(app: &App) -> Option<AppState> {
    app.world()
        .get_resource::<State<AppState>>()
        .map(|state| state.get().clone())
}

/// The spike's per-stack activation tallies (absent only if the plugin never registered).
pub(crate) fn clicks(app: &App) -> Option<UiStackClicks> {
    app.world().get_resource::<UiStackClicks>().copied()
}

/// The single entity carrying a marker component, if there is exactly one.
fn only_entity_with<C: Component>(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<C>>();
    let found: Vec<Entity> = query.iter(world).collect();
    match found.as_slice() {
        [entity] => Some(*entity),
        _ => None,
    }
}

/// The spike's `bevy_ui` button entity.
pub(crate) fn bevy_ui_button(app: &mut App) -> Option<Entity> {
    only_entity_with::<CoexistenceBevyUiButton>(app)
}

/// The entity carrying the primary egui context.
pub(crate) fn egui_context_entity(app: &mut App) -> Option<Entity> {
    only_entity_with::<PrimaryEguiContext>(app)
}

/// The laid-out size of the `bevy_ui` button, if `bevy_ui` has computed one.
pub(crate) fn bevy_ui_button_size(app: &mut App) -> Option<bevy::math::Vec2> {
    let button = bevy_ui_button(app)?;
    app.world()
        .get::<ComputedNode>(button)
        .map(ComputedNode::size)
}

/// Builds the REAL Load-flow app with BOTH UI stacks wired, driven into `AppState::Running`.
///
/// Asserts the descent so every test starts from a known-good two-stack world.
pub(crate) fn app_with_both_ui_stacks() -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_plugins(UiCoexistencePlugin);
    app.add_systems(Update, inject_egui_pointer_click);

    let reached = advance_until(
        &mut app,
        |app| app_state(app) == Some(AppState::Running),
        BUDGET,
    );
    assert!(
        reached,
        "the REAL Load flow must reach AppState::Running within {BUDGET} updates; last observed \
         state was {:?}",
        app_state(&app),
    );
    // A few more frames so `OnEnter(Running)` has spawned the button, `bevy_ui` has laid it out,
    // and `bind_primary_egui_context` has attached the context to the UI camera.
    for _ in 0..8 {
        app.update();
    }
    app
}
