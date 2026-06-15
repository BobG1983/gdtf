use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::{
    BattleRunTurnBudget, BattleRunningComplete,
};

/// `OnExit(BattleScapeState::BattleRunning)`: remove BOTH the per-run completion marker and
/// the per-run turn budget (E10.6 / GTW-208).
///
/// State-scoped-resource discipline (`bevy-traps.md` #1): both resources are per-run, so
/// they are cleaned on exit — a re-entry's `OnEnter` re-inserts a fresh
/// [`BattleRunTurnBudget`] and starts ungated (no leaked marker). A `remove_resource` on an
/// absent resource is a harmless no-op.
pub(in crate::scenes::running::game::battlescape::battle_running) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<BattleRunningComplete>();
    commands.remove_resource::<BattleRunTurnBudget>();
}
