//! GTW-122: headless behavioral tests for the menu button → [`RunningState`]
//! action layer.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] harness (the real
//! state stack + `UiPlugin`, so `FocusNavPlugin` registers the `FocusActivated`
//! message and the `FocusNavSystems::Bridge`/`Apply` sets). They assert on the
//! [`NextState<RunningState>`] the action systems request after an injected
//! activation — the system-effect on the world — never on real device input or
//! rendering.
//!
//! Two activation paths are exercised per enabled button:
//! 1. **Mouse**: set the button's [`Interaction`] to
//!    [`Pressed`](bevy::ui::Interaction::Pressed), `update()`, assert the mapped
//!    transition was requested.
//! 2. **Keyboard / gamepad**: write a [`FocusActivated`] message naming the
//!    button entity, `update()`, assert the same transition.
//!
//! The disabled `HiveScape` button is exercised on BOTH paths and must produce
//! no transition.
//!
//! ## Headless gap
//!
//! Real device input (an actual gamepad South press, an actual mouse click that
//! *produces* `Interaction::Pressed`) is **TBD (Bevy harness)** — there is no
//! window / GPU / HID under `MinimalPlugins`. The bridge that turns `Enter` /
//! gamepad South into a `FocusActivated` is GTW-119's seam and is tested there
//! with synthesized intent; mouse *production* of `Pressed` is GTW-141. Here we
//! drive the action layer's real code path with the synthesized
//! `FocusActivated` message and an injected `Interaction::Pressed`, which is the
//! exact input those upstream seams hand it.

use bevy::{
    ecs::entity::Entity,
    state::state::{NextState, State},
    ui::Interaction,
};
use gdtf_app::test_support::{
    AppState, BattlescapeButton, HiveScapeButton, OptionsButton, QuitButton, RunningState,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{DisabledButton, focus_nav::FocusActivated, theme::default_theme};

/// Builds a headless app driven into [`RunningState::Menu`] with the themed menu
/// spawned (mirrors `menu_scene.rs`'s `menu_app`): seed the theme before the
/// first update so `OnEnter(RunningState::Menu)`'s `spawn_menu` sees it.
fn menu_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.update();
    app
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

/// The state [`NextState<RunningState>`] has been set to, if any. `.set(..)`
/// (used by the action systems) produces [`NextState::Pending`]; the
/// `PendingIfNeq` variant is handled identically so the helper reports whatever
/// target was requested regardless of which setter raised it.
fn pending_state(app: &bevy::app::App) -> Option<RunningState> {
    match app.world().resource::<NextState<RunningState>>() {
        NextState::Pending(state) | NextState::PendingIfNeq(state) => Some(*state),
        NextState::Unchanged => None,
    }
}

/// Sets a button's [`Interaction`] in the world — the swap a real pointer would
/// otherwise drive — so the mouse activation path can be exercised headlessly.
fn set_interaction(app: &mut bevy::app::App, button: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = state;
    }
}

/// Mouse path: pressing each ENABLED button requests its mapped transition —
/// Battlescape→Game, Options→Options, Quit→Quit (AC#1a/b/c, AC#2a).
///
/// Pin: a wrong marker→state mapping, or a dropped `Changed<Interaction>` read,
/// fails the `pending_state` assert. A fresh app per button isolates the
/// `Changed` filter (each is the first press in its world).
#[test]
fn mouse_press_on_enabled_button_requests_mapped_transition() {
    for (label, marker_lookup, expected) in enabled_cases() {
        let mut app = menu_app();
        let button = marker_lookup(&mut app).unwrap_or(Entity::PLACEHOLDER);

        set_interaction(&mut app, button, Interaction::Pressed);
        app.update();

        assert_eq!(
            pending_state(&app),
            Some(expected),
            "mouse press on {label} must request RunningState::{expected:?}",
        );
    }
}

/// Focus-activation path: a `FocusActivated` for each ENABLED button (the Enter /
/// gamepad-South seam) requests its mapped transition (AC#2b).
///
/// Pin: dropping the `FocusActivated` reader, the `.after(Bridge)` ordering, or a
/// wrong mapping fails the assert.
#[test]
fn focus_activation_on_enabled_button_requests_mapped_transition() {
    for (label, marker_lookup, expected) in enabled_cases() {
        let mut app = menu_app();
        let button = marker_lookup(&mut app).unwrap_or(Entity::PLACEHOLDER);

        app.world_mut().write_message(FocusActivated::new(button));
        app.update();

        assert_eq!(
            pending_state(&app),
            Some(expected),
            "focus activation on {label} must request RunningState::{expected:?}",
        );
    }
}

/// Disabled `HiveScape`: NEITHER a mouse press NOR a focus activation produces a
/// transition — it carries `DisabledButton` and is excluded from every action
/// query (AC#1d, AC#2 disabled no-op).
///
/// Pin: if the action queries dropped `Without<DisabledButton>`, the activation
/// would map `HiveScape` to a state and `pending_state` would be `Some`.
#[test]
fn disabled_hivescape_produces_no_transition_on_any_input() {
    // Precondition: HiveScape really is a DisabledButton (guards against the
    // disabled marker silently moving off it).
    let mut app = menu_app();
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    assert!(
        app.world().get::<DisabledButton>(hivescape).is_some(),
        "precondition: HiveScape must carry DisabledButton",
    );

    // Inject BOTH activation paths at once on the HiveScape entity.
    set_interaction(&mut app, hivescape, Interaction::Pressed);
    app.world_mut()
        .write_message(FocusActivated::new(hivescape));
    app.update();

    assert_eq!(
        pending_state(&app),
        None,
        "disabled HiveScape must request no RunningState transition under any input",
    );
    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "and the menu must remain on RunningState::Menu",
    );
}

/// A marker-entity lookup used by [`enabled_cases`] (one of the `single_with::<M>`
/// monomorphizations), named so the case array coerces the function items to a
/// shared pointer type without a per-element `as` cast.
type MarkerLookup = fn(&mut bevy::app::App) -> Option<Entity>;

/// The ENABLED-button cases shared by both path tests: label, marker-entity
/// lookup, and the [`RunningState`] its activation must request.
type EnabledCase = (&'static str, MarkerLookup, RunningState);

/// Battlescape→Game, Options→Options, Quit→Quit — the faithful Godot main-menu
/// map (`HiveScape` is excluded; it is the disabled no-op case).
fn enabled_cases() -> [EnabledCase; 3] {
    [
        (
            "Battlescape",
            single_with::<BattlescapeButton>,
            RunningState::Game,
        ),
        (
            "Options",
            single_with::<OptionsButton>,
            RunningState::Options,
        ),
        ("Quit", single_with::<QuitButton>, RunningState::Quit),
    ]
}
