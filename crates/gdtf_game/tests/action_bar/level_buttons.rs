use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};
use gdtf_game::test_support::{LevelDownButton, LevelUpButton};
use gdtf_test_utils::press_ui_button;

use super::{harness::*, probes::*};

#[test]
fn level_buttons_step_active_level_like_the_intent() {
    let mut app = battle_running_app();
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let Some(up) = require_button::<LevelUpButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, up);
    app.update();
    assert_eq!(
        active_level(&app),
        Some(1),
        "the level-up button raises ActiveLevel by one storey",
    );

    let Some(down) = require_button::<LevelDownButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, down);
    app.update();
    assert_eq!(
        active_level(&app),
        Some(0),
        "the level-down button lowers ActiveLevel by one storey",
    );
}

#[test]
fn level_down_disabled_at_floor() {
    let mut app = battle_running_app();
    set_active_level(&mut app, 0);

    assert!(
        is_disabled::<LevelDownButton>(&mut app),
        "the Level - button must be disabled at the floor storey (0) — a step down is a no-op",
    );
    assert!(
        !is_disabled::<LevelUpButton>(&mut app),
        "the Level + button must NOT be disabled at the floor — a step up is still valid",
    );
}

#[test]
fn level_up_disabled_at_ceiling() {
    let mut app = battle_running_app();
    set_active_level(&mut app, MAX_LEVELS - 1);

    assert!(
        is_disabled::<LevelUpButton>(&mut app),
        "the Level + button must be disabled at the ceiling storey (MAX_LEVELS - 1) — a step up \
         is a no-op",
    );
    assert!(
        !is_disabled::<LevelDownButton>(&mut app),
        "the Level - button must NOT be disabled at the ceiling — a step down is still valid",
    );
}

#[test]
fn neither_level_button_disabled_mid_range() {
    let mut app = battle_running_app();
    set_active_level(&mut app, 1);

    assert!(
        !is_disabled::<LevelDownButton>(&mut app),
        "the Level - button must NOT be disabled mid-range — a step down is valid",
    );
    assert!(
        !is_disabled::<LevelUpButton>(&mut app),
        "the Level + button must NOT be disabled mid-range — a step up is valid",
    );
}

#[test]
fn level_button_disable_tracks_active_level() {
    let mut app = battle_running_app();

    set_active_level(&mut app, 0);
    assert!(
        is_disabled::<LevelDownButton>(&mut app) && !is_disabled::<LevelUpButton>(&mut app),
        "at the floor only Level - is disabled",
    );

    set_active_level(&mut app, 1);
    assert!(
        !is_disabled::<LevelDownButton>(&mut app) && !is_disabled::<LevelUpButton>(&mut app),
        "leaving the floor for a mid storey must RE-ENABLE Level - (and keep Level + enabled)",
    );

    set_active_level(&mut app, MAX_LEVELS - 1);
    assert!(
        is_disabled::<LevelUpButton>(&mut app) && !is_disabled::<LevelDownButton>(&mut app),
        "reaching the ceiling must disable Level + and keep Level - enabled",
    );
}
