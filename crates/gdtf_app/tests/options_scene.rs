//! GTW-637: headless behavioral tests for the `bsn!` + `gdtf_ui`-widgets Options
//! screen pilot.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] harness (the real state
//! stack + `UiPlugin`), driven into [`RunningState::Options`] with the themed screen
//! spawned. They assert on the world effects of the real systems — the spawned
//! markers, the [`NextState<RunningState>`] the Continue button requests, and the
//! sound value readout the toggle adapter mutates — never on real device input or
//! rendering.
//!
//! ## Headless gap
//!
//! Real device input is **TBD (Bevy harness)** — there is no window / GPU under
//! `MinimalPlugins`. For the Continue button the tests inject the
//! [`Interaction::Pressed`] the upstream `ui_focus_system` would write and a
//! synthesized [`FocusActivated`] for the keyboard path — exactly what those producers
//! hand the action layer.
//!
//! For the sound toggle the driven event is the first-party
//! [`bevy_ui_widgets::Checkbox`](bevy::ui_widgets::Checkbox)'s OWN native output —
//! a [`ValueChange<bool>`](bevy::ui_widgets::ValueChange). In the running app the
//! `CheckboxPlugin` (in `DefaultPlugins`) turns a focused `Enter`/`Space` (via
//! [`InputFocus`](bevy::input_focus::InputFocus), NOT `ui_picking`) into that
//! `ValueChange`; that `CheckboxPlugin` is absent under `MinimalPlugins`, so the
//! input-source → `ValueChange` step is exercised by the render-stack screenshot test
//! and upstream `bevy_ui_widgets` tests. Here the test triggers the `ValueChange` the
//! checkbox emits and asserts the screen's integration of it: the first-party
//! `checkbox_self_update` observer flips `Checked` and the screen's `sound_activated`
//! observer folds the typed intent through to the readout.

use bevy::{
    ecs::{entity::Entity, system::RunSystemOnce},
    prelude::{Commands, Text},
    state::state::{NextState, State},
    ui::{Checked, Interaction},
    ui_widgets::ValueChange,
};
use gdtf_app::test_support::{
    AppState, ContinueButton, OptionsScreenRoot, OptionsTitle, RunningState, SoundToggle,
    SoundValueLabel,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{focus_nav::FocusActivated, theme::default_theme};

/// How many updates to allow for a transition / message to settle.
const BUDGET: u32 = 16;

/// Reads the current [`RunningState`].
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// The state [`NextState<RunningState>`] has been set to, if any (mirrors
/// `menu_actions.rs`).
fn pending_state(app: &bevy::app::App) -> Option<RunningState> {
    match app.world().resource::<NextState<RunningState>>() {
        NextState::Pending(state) | NextState::PendingIfNeq(state) => Some(*state),
        NextState::Unchanged => None,
    }
}

/// Looks up the single entity carrying marker `M`, if exactly one exists.
fn single_with<M: bevy::ecs::component::Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Sets an entity's [`Interaction`] in the world — the swap a real pointer would
/// otherwise drive.
fn set_interaction(app: &mut bevy::app::App, entity: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = state;
    }
}

/// Reads the text of the single [`SoundValueLabel`], if present.
fn sound_value_text(app: &mut bevy::app::App) -> Option<String> {
    let entity = single_with::<SoundValueLabel>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|text| text.as_str().to_owned())
}

/// Builds a headless app driven into [`RunningState::Options`] with the themed
/// screen spawned: seed the theme, reach the menu, then stand in for the player
/// selecting Options and wait for the screen to spawn.
fn options_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    // Reach the menu, then queue Menu -> Options (the player's Options click).
    let _ = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    let _ = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Options),
        BUDGET,
    );
    app
}

/// The screen spawns its titled root and its widgets on entry (structure).
#[test]
fn options_screen_spawns_root_title_and_widgets() {
    let mut app = options_app();
    assert_eq!(
        running_state(&app),
        Some(RunningState::Options),
        "precondition: the app must rest in RunningState::Options",
    );
    assert!(
        single_with::<OptionsScreenRoot>(&mut app).is_some(),
        "the Options screen must spawn an OptionsScreenRoot",
    );
    assert!(
        single_with::<OptionsTitle>(&mut app).is_some(),
        "and an OptionsTitle heading",
    );
    assert!(
        single_with::<SoundToggle>(&mut app).is_some(),
        "and the sound-toggle Checkbox (the one first-party widget)",
    );
    assert!(
        single_with::<ContinueButton>(&mut app).is_some(),
        "and a Continue button",
    );
}

/// Mouse path: a press on Continue requests the `Options -> Menu` transition — it
/// returns to the Main Menu it was opened from, NOT on to `Game` (GTW-801).
#[test]
fn continue_mouse_press_returns_to_menu() {
    let mut app = options_app();
    let button = single_with::<ContinueButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    set_interaction(&mut app, button, Interaction::Pressed);
    app.update();
    assert_eq!(
        pending_state(&app),
        Some(RunningState::Menu),
        "a mouse press on Continue must request RunningState::Menu (back to the Main Menu), \
         not RunningState::Game",
    );
}

/// Keyboard / gamepad path: a `FocusActivated` for Continue returns to the Main Menu
/// (`RunningState::Menu`), not on to `Game` (GTW-801).
#[test]
fn continue_focus_activation_returns_to_menu() {
    let mut app = options_app();
    let button = single_with::<ContinueButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(FocusActivated::new(button));
    app.update();
    assert_eq!(
        pending_state(&app),
        Some(RunningState::Menu),
        "a focus activation on Continue must request RunningState::Menu (back to the Main Menu), \
         not RunningState::Game",
    );
}

/// Triggers the sound checkbox's native `ValueChange<bool>` the way a focused
/// `Enter`/`Space` would, with the complement of its current `Checked` state (exactly
/// what `bevy_ui_widgets`' `checkbox_on_key_input` computes).
fn activate_sound_checkbox(app: &mut bevy::app::App, checkbox: Entity) {
    let checked = app.world().get::<Checked>(checkbox).is_some();
    let value = !checked;
    let triggered = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.trigger(ValueChange {
                source: checkbox,
                value,
                is_final: true,
            });
        });
    assert!(
        triggered.is_ok(),
        "the one-shot ValueChange trigger system must run",
    );
}

/// The ONE first-party widget, end to end: the sound Checkbox's native `ValueChange`
/// drives the typed settings intent through to the value readout — "On" becomes "Off" —
/// AND the first-party `checkbox_self_update` observer flips the checkbox's own
/// `Checked` state (the `ValueChange` -> {`checkbox_self_update`, `sound_activated`} ->
/// `SoundSettingChanged` -> `GameSettings` -> label chain).
#[test]
fn sound_toggle_flip_updates_value_label() {
    let mut app = options_app();
    assert_eq!(
        sound_value_text(&mut app).as_deref(),
        Some("On"),
        "sound defaults On, so the readout starts On",
    );

    let checkbox = single_with::<SoundToggle>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    assert!(
        app.world().get::<Checked>(checkbox).is_some(),
        "precondition: the sound checkbox starts Checked (sound On)",
    );

    // The native activation event the checkbox emits on a focused Enter/Space.
    activate_sound_checkbox(&mut app, checkbox);
    // Settle: the ValueChange observers run, then SoundSettingChanged -> GameSettings ->
    // label crosses a couple of system boundaries / message frames. Poll the readout.
    let mut flipped_to_off = false;
    for _ in 0..BUDGET {
        app.update();
        if sound_value_text(&mut app).as_deref() == Some("Off") {
            flipped_to_off = true;
            break;
        }
    }
    assert!(
        flipped_to_off,
        "activating the sound toggle must update the readout to Off within {BUDGET} updates; \
         last observed value was {:?}",
        sound_value_text(&mut app),
    );
    // The first-party checkbox_self_update observer must have cleared Checked in step.
    assert!(
        app.world().get::<Checked>(checkbox).is_none(),
        "the first-party checkbox_self_update observer must clear Checked when toggled Off",
    );
}
