use bevy::{input::ButtonInput, prelude::*};
use cobalt_test_utils::press_key;
use gdtf_battle_input::{ActIntent, BoundKey, Keybinds, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};

use super::harness::*;

fn keyboard_app(active_level: Level) -> App {
    let mut app = selection_app(active_level);
    app.world_mut().insert_resource(test_keybinds());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

pub(crate) fn active_level(app: &App) -> Option<Level> {
    app.world().get_resource::<ActiveLevel>().map(|l| **l)
}

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
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the intent queue must be drained after dispatch",
    );
}

#[test]
fn level_cycling_saturates_and_floors() {
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
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the keyboard-pushed intent must be drained after dispatch",
    );
}

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

#[test]
fn bound_clear_key_press_clears_selection() {
    let level = Level::new(0);
    let mut app = keyboard_app(level);
    let binds = test_keybinds();

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

#[test]
fn unbound_key_press_is_inert() {
    let level = Level::new(2);
    let mut app = keyboard_app(level);
    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));

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
