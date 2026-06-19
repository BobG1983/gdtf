//! Greys the level-step buttons at the storey bounds (GTW-293).
//!
//! The two level buttons ([`LevelUpButton`] / [`LevelDownButton`]) step the presenter's
//! [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) one storey within the valid
//! `0..MAX_LEVELS` range — [`step_level`](gdtf_battle_input::step_level) saturates `Up` at
//! the ceiling storey (`MAX_LEVELS - 1`) and floors `Down` at storey `0`. But a saturated
//! step is silent: at the floor the **Level −** button does nothing, at the ceiling the
//! **Level +** button does nothing, yet both still *look* live. This module makes that
//! visible.
//!
//! [`sync_level_button_bounds`] reactively MUTATES (insert / remove, never despawn /
//! respawn — UI-mutate-in-place) the [`gdtf_ui::DisabledButton`] marker on each level
//! button to match the current [`ActiveLevel`]:
//!
//! - At the FLOOR (`ActiveLevel == 0`) → [`LevelDownButton`] carries `DisabledButton`
//!   (greyed via the disabled theme); above the floor it does NOT.
//! - At the CEILING (`ActiveLevel == MAX_LEVELS - 1`) → [`LevelUpButton`] carries
//!   `DisabledButton`; below the ceiling it does NOT.
//!
//! The same `DisabledButton` marker does double duty (the GTW-309 / GTW-275 precedent): it
//! (a) gives the greyed/disabled look (the `gdtf_ui` disabled-fill paint) AND (b) makes the
//! press router's `Without<DisabledButton>` filter
//! ([`PressedButton`](super::actions::PressedButton)) IGNORE the bounded button — so a press
//! at the bound pushes no intent. The bound is thus enforced visually AND in the input path,
//! not just clamped silently inside `step_level`.

use bevy::prelude::*;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::MAX_LEVELS;
use gdtf_ui::DisabledButton;

use crate::scenes::running::game::battlescape::action_bar::components::{
    LevelDownButton, LevelUpButton,
};

/// Insert-or-remove the [`DisabledButton`] marker on `button` to match `disabled`,
/// MUTATING in place (never a despawn / respawn) — the UI-mutate-in-place convention.
///
/// Only writes on a real change of disabled-state (it checks the current presence first), so
/// it is change-detection-friendly and never churns the entity. Takes the [`Commands`] queue
/// and the existing-disabled flag as a domain bool (the per-button bound test result), so the
/// caller reads "this button is at its bound" once and this helper applies it.
fn set_button_disabled(commands: &mut Commands, button: Entity, currently: bool, disabled: bool) {
    if disabled && !currently {
        commands.entity(button).insert(DisabledButton);
    } else if !disabled && currently {
        commands.entity(button).remove::<DisabledButton>();
    }
}

/// Greys (disables) each level button at its storey bound and re-enables it off the bound,
/// reacting to the live [`ActiveLevel`] (GTW-293).
///
/// Runs every update under the live-battle gate; it is cheap (two single-entity queries) and
/// idempotent — [`set_button_disabled`] only writes on a real disabled-state change, so a
/// resting level produces no churn. It covers BOTH the spawn frame (the bar spawns at
/// `OnEnter(BattleRunning)` with `ActiveLevel` defaulting to the ground floor, so the **Level
/// −** button is greyed from the first frame) AND every subsequent level step.
///
/// The bounds: the floor is storey `0` and the ceiling is `MAX_LEVELS - 1` (the same clamp
/// [`step_level`](gdtf_battle_input::step_level) saturates to). `saturating_sub` guards the
/// impossible `MAX_LEVELS == 0`.
///
/// Param-only (`bevy-traps.md` #7): a `Res<ActiveLevel>` read, two read-only marker queries
/// (each carrying `Has<DisabledButton>` so the current state is read without a second query),
/// and a [`Commands`] queue for the insert / remove — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_level_button_bounds(
    mut commands: Commands,
    active: Res<ActiveLevel>,
    level_up: Query<(Entity, Has<DisabledButton>), With<LevelUpButton>>,
    level_down: Query<(Entity, Has<DisabledButton>), With<LevelDownButton>>,
) {
    let storey = *(**active);
    // The top valid storey index (the same ceiling `step_level` saturates to). `MAX_LEVELS`
    // is 8, so the ceiling is storey 7; `saturating_sub` guards the impossible `MAX_LEVELS == 0`.
    let ceiling = MAX_LEVELS.saturating_sub(1);

    // At the ceiling, a further step Up is a no-op → grey the **Level +** button.
    let at_ceiling = storey >= ceiling;
    for (button, currently) in &level_up {
        set_button_disabled(&mut commands, button, currently, at_ceiling);
    }

    // At the floor, a further step Down is a no-op → grey the **Level −** button.
    let at_floor = storey == 0;
    for (button, currently) in &level_down {
        set_button_disabled(&mut commands, button, currently, at_floor);
    }
}
