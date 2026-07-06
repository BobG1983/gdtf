//! GTW-293 level up/down buttons: stepping + disabled-at-bounds.

use gdtf_app::test_support::{LevelDownButton, LevelUpButton};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{metric::MAX_LEVELS, prelude::Level};
use gdtf_test_utils::press_ui_button;

use super::{harness::*, probes::*};

/// AC3 — pressing the level-up button raises `ActiveLevel` by one storey (the same
/// `ActiveLevel` mutation the `LevelUp` intent drives), and level-down lowers it.
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

// ---------------------------------------------------------------------------------
// GTW-293 — the level buttons are VISUALLY disabled (greyed via the `gdtf_ui`
// `DisabledButton` marker) at the storey bounds: **Level −** at the floor (0), **Level +**
// at the ceiling (`MAX_LEVELS - 1`), and NEITHER at a mid storey. The same marker disables
// the look AND makes the press router's `Without<DisabledButton>` filter ignore the bounded
// button, so the bound is enforced visually + in the input path, not just clamped in
// `step_level`. Drives the REAL `sync_level_button_bounds` system by mutating `ActiveLevel`
// and settling an update.
// ---------------------------------------------------------------------------------

/// GTW-293 — at the FLOOR storey (0), the **Level −** button is disabled and the **Level +**
/// button is NOT (a step Down is a no-op there; a step Up is still valid).
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

/// GTW-293 — at the CEILING storey (`MAX_LEVELS - 1`), the **Level +** button is disabled
/// and the **Level −** button is NOT (a step Up is a no-op there; a step Down is still valid).
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

/// GTW-293 — at a MID storey (neither floor nor ceiling), NEITHER level button is disabled
/// (both steps are valid). The discriminating case: a system that always disabled (or never
/// disabled) would fail one of the three tests.
#[test]
fn neither_level_button_disabled_mid_range() {
    let mut app = battle_running_app();
    // A storey strictly between the floor (0) and the ceiling (MAX_LEVELS - 1). MAX_LEVELS is
    // 8, so storey 1 is safely mid-range (0 < 1 < 7).
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

/// GTW-293 — the disable state is REACTIVE: re-enabled when `ActiveLevel` leaves the bound.
/// Drive floor → mid → ceiling and assert the markers move (proving `sync_level_button_bounds`
/// removes `DisabledButton` off the bound, not just inserts it once — UI-mutate-in-place).
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
