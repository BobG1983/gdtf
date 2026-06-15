use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::BattleRunTurnBudget;

/// `FixedUpdate` (gated `in_state(BattleRunning)`): spend one tick of the per-run
/// [`BattleRunTurnBudget`], **saturating** at zero (E10.6 / GTW-208).
///
/// Decrements the budget once per `FixedUpdate` tick the phase is active (the harness's
/// `FixedTimesteps(1)` makes that exactly one tick per `App::update()`, so the gate is
/// deterministic). The decrement saturates at zero
/// ([`BattleRunTurnBudget::tick`]) — it never wraps below zero — so the budget rests at zero
/// once spent and the completion gate stays satisfied. Gated on
/// [`BattleRunTurnBudget`]'s presence via `Option<ResMut<…>>`, so it is a no-op (no panic)
/// when the budget is absent (`bevy-traps.md` #1).
pub(in crate::scenes::running::game::battlescape::battle_running) fn decrement_turn_budget(
    budget: Option<ResMut<BattleRunTurnBudget>>,
) {
    let Some(mut budget) = budget else {
        return;
    };
    *budget = budget.tick();
}
