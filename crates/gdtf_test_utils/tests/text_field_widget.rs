//! GTW-411 headless integration test for the [`TextField`](gdtf_ui::TextField) /
//! [`NumericField`](gdtf_ui::NumericField) widgets, driving the REAL keyboard observer + commit
//! path on the REAL `bevy_ui` layout (verification rule 3).
//!
//! These run on the [`GdtfUiTestAppBuilder`] `DefaultPlugins` headless harness with
//! [`gdtf_ui::UiPlugin`], `register_text_field`, and `register_numeric_field::<i64>` added, so
//! the field observers + commit systems actually run.
//!
//! ## How the keyboard is driven — the REAL device-dispatch bridge
//!
//! These tests drive the FULL production keypress path: `bevy_input_focus`'s
//! `dispatch_focused_input::<KeyboardInput>` (in `DefaultPlugins`, `PreUpdate`) reads the
//! [`KeyboardInput`](bevy::input::keyboard::KeyboardInput) message stream and TRIGGERS a
//! [`FocusedInput<KeyboardInput>`](bevy::input_focus::FocusedInput) entity-event at the focused
//! entity, which the [`handle_text_field_key`](gdtf_ui::handle_text_field_key) observer reads.
//!
//! That dispatch needs a `PrimaryWindow` (`windows.single()`), and the headless UI harness
//! builds with `primary_window: None` — so each test spawns its OWN
//! [`PrimaryWindow`](bevy::window::PrimaryWindow) entity (a pure ECS entity; no OS window is
//! needed, the dispatch only QUERIES for it). The tests then set
//! [`InputFocus`](bevy::input_focus::InputFocus), WRITE a real `KeyboardInput` message, and run
//! `PreUpdate` + `Update`, so the entire bridge → observer → commit chain runs unmodified.

use bevy::{
    ecs::system::SystemState,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, FocusLost, InputFocus},
    prelude::*,
    ui::Interaction,
    window::PrimaryWindow,
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use gdtf_ui::{
    CommittedTextValue, FieldColors, NumericFieldCommitted, NumericRange, TextField,
    TextFieldCommitted, UiPlugin, register_numeric_field, register_text_field, spawn_numeric_field,
    spawn_text_field,
};

/// A distinct test color set so a render of the field is unambiguous.
const COLORS: FieldColors = FieldColors {
    background: Color::srgb(0.14, 0.14, 0.18),
    text:       Color::srgb(0.9, 0.9, 0.8),
    caret:      Color::srgb(0.95, 0.85, 0.30),
};

/// The inclusive numeric range the clamp tests use.
const RANGE: NumericRange<i64> = NumericRange::new(1, 10);

/// Builds the harness: the headless UI app + `UiPlugin` + the field registrations, so the real
/// observers/systems run on a real layout.
fn harness() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    register_text_field(&mut app);
    register_numeric_field::<i64>(&mut app);
    // The headless UI harness builds with `primary_window: None`, but
    // `dispatch_focused_input::<KeyboardInput>` only fires when a `PrimaryWindow` exists. Spawn
    // a pure-ECS one (no OS window) so the REAL keypress-dispatch bridge runs.
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    app
}

/// Spawns a text field seeded with `initial` and settles the layout, returning its entity.
fn spawn_test_text_field(app: &mut App, initial: &str) -> Entity {
    let field = {
        let mut commands = app.world_mut().commands();
        spawn_text_field(&mut commands, CommittedTextValue::new(initial), COLORS, ())
    };
    for _ in 0..3 {
        app.update();
    }
    field
}

/// Spawns a numeric field seeded with `initial` (clamped into [`RANGE`]) and settles, returning
/// its entity.
fn spawn_test_numeric_field(app: &mut App, initial: i64) -> Entity {
    let field = {
        let mut commands = app.world_mut().commands();
        spawn_numeric_field::<i64>(&mut commands, initial, RANGE, COLORS, ())
    };
    for _ in 0..3 {
        app.update();
    }
    field
}

/// Focuses `field` via [`InputFocus`] (the production precondition for keyboard dispatch).
fn focus(app: &mut App, field: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
}

/// Writes a real [`KeyboardInput`] message for a typed character + runs the bridge so
/// `dispatch_focused_input` triggers the observer, accumulating the char into the focused
/// field's buffer; then `Update` so the same-frame `sync_edit_buffer_to_text` redraws.
fn type_char(app: &mut App, ch: &str) {
    let window = window_of(app);
    write_key(
        app,
        KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(ch.into()),
            state: ButtonState::Pressed,
            text: Some(ch.into()),
            repeat: false,
            window,
        },
    );
}

/// Writes a NAMED-key (Enter / Backspace / Escape) [`KeyboardInput`] message and runs the
/// bridge; the dispatch flow runs twice so a queued numeric-commit command flushes and its
/// message is readable before the asserts.
fn press_named_key(app: &mut App, key: Key) {
    let window = window_of(app);
    write_key(
        app,
        KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
}

/// The spawned primary-window entity (the dispatch targets the window when no entity is
/// focused; a `KeyboardInput` carries its origin window).
fn window_of(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    q.single(app.world()).unwrap_or(Entity::PLACEHOLDER)
}

/// Writes one [`KeyboardInput`] message then runs `PreUpdate` (where
/// `dispatch_focused_input::<KeyboardInput>` reads the message + triggers the observer) and
/// `Update` (where `sync_edit_buffer_to_text` and any queued commit flush). It runs the
/// `PreUpdate`/`Update` pair, then one extra `Update`, so a numeric commit queued by the
/// observer (a deferred command) lands its message in the buffer the asserts read.
fn write_key(app: &mut App, input: KeyboardInput) {
    app.world_mut().write_message(input);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Update);
}

/// The shown text of the field's `FieldText` child.
fn shown_text(app: &App, field: Entity) -> Option<String> {
    let children = app.world().get::<Children>(field)?;
    for &child in children {
        if app.world().get::<gdtf_ui::FieldText>(child).is_some() {
            return app.world().get::<Text>(child).map(|t| t.0.clone());
        }
    }
    None
}

/// Drains the `TextFieldCommitted` messages.
fn text_commits(app: &mut App) -> Vec<TextFieldCommitted> {
    let mut state: SystemState<MessageReader<TextFieldCommitted>> =
        SystemState::new(app.world_mut());
    let Ok(mut reader) = state.get_mut(app.world_mut()) else {
        return Vec::new();
    };
    reader.read().cloned().collect()
}

/// Drains the `NumericFieldCommitted<i64>` messages.
fn numeric_commits(app: &mut App) -> Vec<NumericFieldCommitted<i64>> {
    let mut state: SystemState<MessageReader<NumericFieldCommitted<i64>>> =
        SystemState::new(app.world_mut());
    let Ok(mut reader) = state.get_mut(app.world_mut()) else {
        return Vec::new();
    };
    reader.read().copied().collect()
}

/// T1 (C1) — focusing a text field and DRIVING REAL keyboard input through the
/// [`FocusedInput<KeyboardInput>`] observer accumulates the typed characters into the
/// [`EditBuffer`], the shown [`Text`] mirrors it, and pressing Enter emits a typed
/// [`TextFieldCommitted`] carrying the edited [`CommittedTextValue`] + the field entity.
///
/// Pin-discriminating: a revert of the keyboard observer leaves the buffer empty (the typed
/// chars never land), failing the shown-text assert; a revert of the commit path emits no
/// message, failing the message asserts.
#[test]
fn typing_then_enter_commits_typed_text_value() {
    let mut app = harness();
    let field = spawn_test_text_field(&mut app, "");
    focus(&mut app, field);

    // Type "Esher" one character at a time through the real observer.
    for ch in ["E", "s", "h", "e", "r"] {
        type_char(&mut app, ch);
    }

    // The shown text mirrors the accumulated buffer (the observer + sync ran).
    assert_eq!(
        shown_text(&app, field).as_deref(),
        Some("Esher"),
        "typed characters must accumulate in the buffer and render in the shown text",
    );

    // Backspace pops the last character (end-only editing).
    press_named_key(&mut app, Key::Backspace);
    assert_eq!(
        shown_text(&app, field).as_deref(),
        Some("Eshe"),
        "Backspace pops the last typed character",
    );

    // Enter commits the edited value as the typed newtype.
    drop(text_commits(&mut app)); // clear any prior
    press_named_key(&mut app, Key::Enter);

    let msgs = text_commits(&mut app);
    assert_eq!(msgs.len(), 1, "Enter emits exactly one text-commit message");
    assert_eq!(
        msgs[0].value().value(),
        "Eshe",
        "the commit carries the edited text as CommittedTextValue",
    );
    assert_eq!(
        msgs[0].field(),
        field,
        "the commit carries the field's identity",
    );
}

/// T2 (C2) — a numeric field CLAMPS its committed value to the provided range: typing a value
/// ABOVE the max commits the max, and a value BELOW the min commits the min.
///
/// Pin-discriminating against a no-clamp: without [`NumericRange::clamp`] the commit would carry
/// the raw out-of-range value (`50` / `-5`), failing both asserts.
#[test]
fn numeric_field_clamps_committed_value_to_range() {
    let mut app = harness();

    // Above-max: type "50", Enter -> clamps to 10.
    let field = spawn_test_numeric_field(&mut app, 5);
    focus(&mut app, field);
    // Replace the seeded "5" with "50": backspace it then type 5, 0.
    press_named_key(&mut app, Key::Backspace);
    type_char(&mut app, "5");
    type_char(&mut app, "0");
    drop(numeric_commits(&mut app));
    press_named_key(&mut app, Key::Enter);

    let msgs = numeric_commits(&mut app);
    assert_eq!(msgs.len(), 1, "Enter emits exactly one numeric-commit");
    assert_eq!(
        msgs[0].value().value(),
        10,
        "an above-max value clamps to the range max",
    );

    // Below-min: a fresh field, type "-5", Enter -> clamps to 1.
    let mut app = harness();
    let field = spawn_test_numeric_field(&mut app, 5);
    focus(&mut app, field);
    press_named_key(&mut app, Key::Backspace);
    type_char(&mut app, "-");
    type_char(&mut app, "5");
    drop(numeric_commits(&mut app));
    press_named_key(&mut app, Key::Enter);

    let msgs = numeric_commits(&mut app);
    assert_eq!(msgs.len(), 1, "Enter emits exactly one numeric-commit");
    assert_eq!(
        msgs[0].value().value(),
        1,
        "a below-min value clamps to the range min",
    );
}

/// T3 (C3) — an INVALID / empty numeric buffer is REJECTED (reverted to the last-good value) on
/// Enter with NO panic and NO bare-type leak.
///
/// The test running to completion + the asserts passing proves the parse path never panicked.
/// Pin-discriminating: a naive `parse().unwrap()` would panic on "abc" (failing the run); a
/// "commit anyway" path would emit a garbage value rather than the last-good `5`.
#[test]
fn invalid_numeric_input_reverts_without_panic() {
    let mut app = harness();
    let field = spawn_test_numeric_field(&mut app, 5);
    focus(&mut app, field);

    // Clear the seeded "5" and type a non-numeric buffer.
    press_named_key(&mut app, Key::Backspace);
    for ch in ["a", "b", "c"] {
        type_char(&mut app, ch);
    }
    assert_eq!(
        shown_text(&app, field).as_deref(),
        Some("abc"),
        "precondition: the invalid buffer is shown before commit",
    );

    drop(numeric_commits(&mut app));
    press_named_key(&mut app, Key::Enter);

    // The commit reverts to the last-good value (5) — never panics, never leaks a raw value.
    let msgs = numeric_commits(&mut app);
    assert_eq!(msgs.len(), 1, "Enter still emits one numeric-commit");
    assert_eq!(
        msgs[0].value().value(),
        5,
        "an invalid/empty buffer reverts to the last-good committed value",
    );
    // And the buffer normalized back to the committed value's display form.
    assert_eq!(
        shown_text(&app, field).as_deref(),
        Some("5"),
        "the shown text normalizes to the reverted last-good value",
    );

    // Empty buffer case: clear everything, Enter -> still reverts to last-good (5), no panic.
    press_named_key(&mut app, Key::Backspace);
    assert_eq!(
        shown_text(&app, field).as_deref(),
        Some(""),
        "precondition: the buffer is now empty",
    );
    drop(numeric_commits(&mut app));
    press_named_key(&mut app, Key::Enter);
    let msgs = numeric_commits(&mut app);
    assert_eq!(
        msgs.first().map(|m| m.value().value()),
        Some(5),
        "an EMPTY buffer also reverts to the last-good value without panic",
    );
}

/// Focus test — clicking a field (its [`Interaction`] going `Pressed`) sets
/// [`InputFocus`](bevy::input_focus::InputFocus) to it, so the keyboard observer then targets
/// it (strengthens discrimination of the click-to-focus path).
///
/// Pin-discriminating: a revert of [`focus_field_on_press`](gdtf_ui::focus_field_on_press)
/// leaves `InputFocus` unset, failing the assert.
#[test]
fn pressing_a_field_focuses_it() {
    let mut app = harness();
    let field = spawn_test_text_field(&mut app, "x");

    // Drive the press edge then run Update (where focus_field_on_press lives).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(field) {
        *interaction = Interaction::Pressed;
    }
    app.world_mut().run_schedule(Update);

    let focused = app.world().resource::<InputFocus>().get();
    assert_eq!(
        focused,
        Some(field),
        "pressing a field must set InputFocus to it so keyboard input reaches it",
    );
    // And it is genuinely a TextField (not some other focusable).
    assert!(
        app.world().get::<TextField>(field).is_some(),
        "the focused entity is the text field",
    );
}

/// Commit-on-blur — losing focus ([`FocusLost`]) commits a text field's edited value (the
/// menu-form expectation: clicking away still reports the value).
///
/// Pin-discriminating: a revert of [`commit_on_focus_lost`](gdtf_ui::commit_on_focus_lost)
/// emits no message on blur, failing the assert.
#[test]
fn losing_focus_commits_text_field() {
    let mut app = harness();
    let field = spawn_test_text_field(&mut app, "");
    focus(&mut app, field);
    for ch in ["V", "a", "n"] {
        type_char(&mut app, ch);
    }
    drop(text_commits(&mut app));

    // Trigger the real FocusLost entity-event the focus framework emits on a focus change.
    app.world_mut().trigger(FocusLost { entity: field });
    app.world_mut().run_schedule(Update);

    let msgs = text_commits(&mut app);
    assert_eq!(msgs.len(), 1, "losing focus commits the edited value");
    assert_eq!(
        msgs[0].value().value(),
        "Van",
        "the blur-commit carries the edited text",
    );
}
