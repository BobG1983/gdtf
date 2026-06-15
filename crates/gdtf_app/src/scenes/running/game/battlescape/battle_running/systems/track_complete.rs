use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::{
    BattleRunTurnBudget, BattleRunningComplete,
};

/// The starting [`BattleRunTurnBudget`] inserted `OnEnter(BattleRunning)`.
///
/// A SMALL count of `FixedUpdate` ticks (a few): the gate fires well within the
/// `state_walk` `WALK_BUDGET` (64 updates) so the deep headless walk still descends
/// `BattleRunning → AnimateOut → … → Teardown` (E10.6 AC5). It is a placeholder for the
/// headless bootstrap, not a balance value — the real turn loop (E9/GTW-14) supersedes it.
const STARTING_TURN_BUDGET: BattleRunTurnBudget = BattleRunTurnBudget::new(3);

/// `OnEnter(BattleScapeState::BattleRunning)`: insert the per-run [`BattleRunTurnBudget`].
///
/// Seeds the finite, deterministic completion gate with [`STARTING_TURN_BUDGET`]. The
/// budget is a per-run state-scoped resource (`bevy-traps.md` #1): inserted here on entry
/// and removed by `cleanup` on exit, so a re-entry starts from a fresh budget.
pub(in crate::scenes::running::game::battlescape::battle_running) fn insert_turn_budget(
    mut commands: Commands,
) {
    commands.insert_resource(STARTING_TURN_BUDGET);
}

/// `FixedUpdate` (gated `in_state(BattleRunning)`): mark the phase complete once the
/// [`BattleRunTurnBudget`] is spent.
///
/// Replaces the previous UNCONDITIONAL `BattleRunningComplete` insert (the instant
/// entry-tick race): it now inserts the marker exactly once the per-run budget has reached
/// zero via its saturating per-tick decrement ([`decrement_turn_budget`]). The plugin gates
/// this system on `not(resource_exists::<BattleRunningComplete>)`, so it inserts the marker
/// at most once; the existing `move_on` (gated on the marker) then advances `BattleRunning →
/// AnimateOut`. A missing budget (it is state-scoped) leaves the gate inert — the
/// `Option<Res<…>>` guard never panics (`bevy-traps.md` #1).
pub(in crate::scenes::running::game::battlescape::battle_running) fn game_battlescape_battle_running_complete(
    budget: Option<Res<BattleRunTurnBudget>>,
    mut commands: Commands,
) {
    let Some(budget) = budget else {
        return;
    };
    if budget.is_spent() {
        commands.insert_resource(BattleRunningComplete);
    }
}
