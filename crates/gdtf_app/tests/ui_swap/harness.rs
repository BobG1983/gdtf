//! Shared setup for the GTW-816 swap-harness suite: the REAL Load flow with the REAL
//! `UiSwapHarnessPlugin` added, driven to `AppState::Running`, plus the readers and the two
//! input injectors the tests drive it with.
//!
//! # Why the egui click is injected as raw input
//!
//! The headless harness runs with `primary_window: None` (no OS window, no cursor), so
//! `bevy_egui`'s own input systems have no pointer to read — the ONE thing a windowless app
//! cannot supply. [`inject_egui_pointer_click`] therefore writes the pointer events into the
//! context's [`EguiInput`] (the exact buffer `bevy_egui`'s `write_egui_input_system` fills
//! from window events), during `Update`, so `run_egui_context_pass_loop_system` takes them
//! in `PostUpdate` the way it takes real ones. Everything downstream is the REAL path: egui
//! hit-tests the swap button's rect itself and the production `ui.button(..).clicked()`
//! branch is what fires. Nothing about the harness's systems is stubbed.

use bevy::{
    app::{App, Update},
    ecs::{entity::Entity, resource::Resource},
    input::{
        ButtonState,
        keyboard::{Key, KeyCode, KeyboardInput, NativeKey},
    },
    prelude::{Commands, Component, Query, Res, With},
    state::state::State,
};
use bevy_egui::{EguiInput, PrimaryEguiContext, egui};
use gdtf_app::test_support::{
    AppState, EGUI_SWAP_PANEL_POS, UI_STACK_SWAP_KEY, UiStack, UiStackId, UiSwapBevyUiButton,
    UiSwapBevyUiPanel, UiSwapHarnessPlugin,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads plus the state descent
/// into `Running` under contention (the GTW-819 suite's precedent).
pub(crate) const BUDGET: u32 = 512;

/// How many frames a queued egui click needs to reach the live stack: one `Update` to
/// inject the pointer events, that frame's `PostUpdate` egui pass to latch, and the NEXT
/// `Update` to apply the latch. A couple of spare frames keep the assertion off a
/// one-frame cliff — and, crucially, give a DOUBLE-firing swap every chance to show itself.
pub(crate) const EGUI_CLICK_FRAMES: usize = 4;

/// The screen extent handed to egui, in points.
///
/// `bevy_egui` normally derives this from the primary window; there is none here, and an
/// egui context with no screen rect clips every widget away — so the harness supplies one
/// big enough to contain the swap panel.
const HEADLESS_SCREEN: egui::Vec2 = egui::vec2(1280.0, 720.0);

/// Requests one synthetic egui pointer click at the egui swap panel's pinned position.
///
/// A fieldless marker resource: a test inserts it, [`inject_egui_pointer_click`] consumes it
/// on the next `Update`, so the click is delivered on a known frame.
#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct PendingEguiClick;

/// `Update`: deliver a queued pointer press+release to the primary egui context.
///
/// Aims at the interior of the swap button — the button is the area's first widget, so its
/// top-left is `EGUI_SWAP_PANEL_POS` (see the production module's doc) and a small inset
/// lands inside its rect.
pub(crate) fn inject_egui_pointer_click(
    pending: Option<Res<PendingEguiClick>>,
    mut contexts: Query<&mut EguiInput, With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    if pending.is_none() {
        return;
    }
    let pos = egui::pos2(EGUI_SWAP_PANEL_POS.x + 8.0, EGUI_SWAP_PANEL_POS.y + 8.0);
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

/// Deliver a synthetic egui pointer click and settle it through the latch.
///
/// Runs a couple of plain frames FIRST: egui lays a newly-shown area out on the frame it
/// first draws it, so a pointer aimed at a panel that has never been drawn lands before the
/// rect it is aiming at exists. Settling first makes the click a click on a panel that is
/// already on screen, which is what a user does.
pub(crate) fn click_egui_swap_button(app: &mut App) {
    for _ in 0..2 {
        app.update();
    }
    app.world_mut().insert_resource(PendingEguiClick);
    for _ in 0..EGUI_CLICK_FRAMES {
        app.update();
    }
}

/// Tap the harness's SHIPPED swap key through Bevy's real windowing-input path.
///
/// Writes a `KeyboardInput` press+release pair onto the buffered keyboard stream — exactly
/// what the backend writes — so `keyboard_input_system` folds it into
/// `ButtonInput<KeyCode>` and the production `ui_stack_swap_key` reads a real
/// `just_pressed`. The key itself is the shipped `UI_STACK_SWAP_KEY` const, never a literal
/// copy of it, so a rebinding cannot leave this test passing against a stale key.
pub(crate) fn tap_swap_key(app: &mut App) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: UI_STACK_SWAP_KEY,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }
}

/// The key code a tap of the shipped shortcut sends, for a test that wants to assert an
/// UNRELATED key changes nothing.
pub(crate) const AN_UNRELATED_KEY: KeyCode = KeyCode::KeyZ;

/// Tap an arbitrary key through the same real path as [`tap_swap_key`].
pub(crate) fn tap_key(app: &mut App, key_code: KeyCode) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }
}

/// The current [`AppState`], if the state stack is up.
pub(crate) fn app_state(app: &App) -> Option<AppState> {
    app.world()
        .get_resource::<State<AppState>>()
        .map(|state| state.get().clone())
}

/// Which UI stack the harness currently has live (absent only if it never registered).
pub(crate) fn live_stack(app: &App) -> Option<UiStackId> {
    app.world().get_resource::<UiStack>().map(|s| s.live())
}

/// How many entities carry a marker component.
pub(crate) fn count_with<C: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<C>>();
    query.iter(world).count()
}

/// The `bevy_ui` rendering's swap button entity, if it is on screen.
pub(crate) fn bevy_ui_swap_button(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<UiSwapBevyUiButton>>();
    let found: Vec<Entity> = query.iter(world).collect();
    match found.as_slice() {
        [entity] => Some(*entity),
        _ => None,
    }
}

/// How many entities the `bevy_ui` rendering currently has on screen (its panel root).
pub(crate) fn bevy_ui_panel_count(app: &mut App) -> usize {
    count_with::<UiSwapBevyUiPanel>(app)
}

/// Builds the REAL Load-flow app with the REAL swap harness wired, driven to
/// `AppState::Running` with both stacks available and the `bevy_ui` one live.
///
/// Asserts the descent so every test starts from a known-good world.
pub(crate) fn app_with_swap_harness() -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_plugins(UiSwapHarnessPlugin);
    app.add_systems(Update, inject_egui_pointer_click);

    let reached = advance_until(
        &mut app,
        |app| app_state(app) == Some(AppState::Running),
        BUDGET,
    );
    assert!(
        reached,
        "the REAL Load flow must reach AppState::Running within {BUDGET} updates; last \
         observed state was {:?}",
        app_state(&app),
    );
    // A few more frames so `OnEnter(Running)` has spawned the panel, `bevy_ui` has laid it
    // out, and `bind_primary_egui_context` has attached the context to the UI camera.
    for _ in 0..8 {
        app.update();
    }
    app
}
