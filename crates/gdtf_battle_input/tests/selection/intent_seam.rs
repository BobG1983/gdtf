//! Level cycling + selection clear through the act-intent queue, bound and
//! unbound keys (AC6/AC9).

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{ActIntent, BoundKey, Keybinds, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};
use gdtf_test_utils::press_key;

use super::harness::*;

/// Builds a selection app that ALSO has the keyboard press surface live: the
/// asset-loaded [`Keybinds`] table is inserted DIRECTLY (the keybinds.rs docs'
/// headless idiom — "a test that wants them inserts `Keybinds`
/// directly") and an empty [`ButtonInput<KeyCode>`] is seeded so `level_keys` /
/// `select_clear_key` (which `Res`-read that buffer) run instead of failing param
/// validation under `MinimalPlugins` (no `InputPlugin`).
fn keyboard_app(active_level: Level) -> App {
    let mut app = selection_app(active_level);
    app.world_mut().insert_resource(test_keybinds());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

/// A fully-bound [`Keybinds`] table for the keyboard real-path tests — every act is
/// on a distinct [`BoundKey`]. Built in the test body (not parsed from the shipped
/// `.ron`) so the keyboard tests do not depend on the editable file's chosen keys.
const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        // GTW-521 — the full-view toggle key.
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        // GTW-458 — the Tab/Shift+Tab Prev/Next cycle chord (both bind to Tab).
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

/// The current `ActiveLevel`.
pub(crate) fn active_level(app: &App) -> Option<Level> {
    app.world().get_resource::<ActiveLevel>().map(|l| **l)
}

// ---------------------------------------------------------------------------------
// AC6/AC9 — level cycling THROUGH the shared intent queue.
// ---------------------------------------------------------------------------------

/// AC9 + AC6 — writing a `LevelUp` intent and updating drains it through the ONE
/// `dispatch_act_intents` system and mutates `ActiveLevel` (the queue is real).
#[test]
fn level_up_intent_raises_active_level_through_the_seam() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    push_intent(&mut app, ActIntent::LevelUp);
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(1)),
        "a LevelUp intent must raise ActiveLevel by one storey through the drain",
    );
    // The queue is drained (empty) after dispatch.
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the intent queue must be drained after dispatch",
    );
}

/// AC6 — level-up SATURATES at `MAX_LEVELS - 1` and level-down FLOORS at 0, through
/// the queue.
#[test]
fn level_cycling_saturates_and_floors() {
    // Up from the top storey stays at the top.
    {
        let top = Level::new(MAX_LEVELS - 1);
        let mut app = selection_app(top);
        push_intent(&mut app, ActIntent::LevelUp);
        app.update();
        assert_eq!(
            active_level(&app),
            Some(top),
            "LevelUp must saturate at MAX_LEVELS - 1",
        );
    }
    // Down from the ground floor stays at 0.
    {
        let ground = Level::new(0);
        let mut app = selection_app(ground);
        push_intent(&mut app, ActIntent::LevelDown);
        app.update();
        assert_eq!(
            active_level(&app),
            Some(ground),
            "LevelDown must floor at 0",
        );
    }
}

/// AC9 — a `SelectionClear` intent through the queue clears `SelectedShooter`.
#[test]
fn selection_clear_intent_clears_through_the_seam() {
    let level = Level::new(0);
    let mut app = selection_app(level);
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    push_intent(&mut app, ActIntent::SelectionClear);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a SelectionClear intent must clear the selection through the drain",
    );
}

// ---------------------------------------------------------------------------------
// AC6 (keyboard real path) — a bound KEY press drives the keyboard systems
// end-to-end: level_keys / select_clear_key -> intent -> dispatch -> state.
// ---------------------------------------------------------------------------------

/// AC6 — pressing the BOUND level-up key drives the REAL `level_keys` system: the
/// just-pressed `KeyCode` -> `ActIntent::LevelUp` push -> `dispatch_act_intents`
/// drain -> `ActiveLevel` rises one storey. Reverting `level_keys` (so no intent is
/// pushed) fails this — the key-press->ActiveLevel path is exercised, not bypassed.
#[test]
fn bound_level_up_key_press_raises_active_level() {
    let level = Level::new(0);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    press_key(&mut app, binds.level_up());
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(1)),
        "a press of the bound level-up key must raise ActiveLevel through level_keys",
    );
    // The intent the keyboard system pushed was drained by dispatch this update.
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the keyboard-pushed intent must be drained after dispatch",
    );
}

/// AC6 — pressing the BOUND level-DOWN key drives `level_keys` the other way:
/// from an upper storey it lowers `ActiveLevel` by one (proving the down branch of
/// the real keyboard system runs, not just the up one).
#[test]
fn bound_level_down_key_press_lowers_active_level() {
    let level = Level::new(3);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    press_key(&mut app, binds.level_down());
    app.update();

    assert_eq!(
        active_level(&app),
        Some(Level::new(2)),
        "a press of the bound level-down key must lower ActiveLevel through level_keys",
    );
}

/// AC6 — pressing the BOUND clear key drives the REAL `select_clear_key` system:
/// the just-pressed `KeyCode` -> `ActIntent::SelectionClear` push -> drain ->
/// `SelectedShooter` cleared. Reverting `select_clear_key` fails this.
#[test]
fn bound_clear_key_press_clears_selection() {
    let level = Level::new(0);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

    // Pre-seed a selection, then press the bound clear key.
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    press_key(&mut app, binds.select_clear());
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a press of the bound clear key must clear the selection through select_clear_key",
    );
}

/// AC6 — an UNBOUND key press does nothing: pressing a key no act is bound to
/// leaves `ActiveLevel` and `SelectedShooter` untouched (the keyboard systems read
/// the bound key off `Keybinds`, never a hardcoded literal — a stray press is inert).
#[test]
fn unbound_key_press_is_inert() {
    let level = Level::new(2);
    let mut app = keyboard_app(level);
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

    // `Space` is bound to no act in `test_keybinds`.
    press_key(&mut app, KeyCode::Space);
    app.update();

    assert_eq!(
        active_level(&app),
        Some(level),
        "an unbound key press must not change ActiveLevel",
    );
    assert_eq!(
        selected(&app),
        Some(ganger),
        "an unbound key press must not clear the selection",
    );
}
