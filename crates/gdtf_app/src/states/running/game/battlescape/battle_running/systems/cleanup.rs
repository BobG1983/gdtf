use bevy::prelude::*;

use crate::states::running::game::battlescape::battle_running::{
    resources::BattleRunningComplete, systems::EndTransition,
};

/// `OnExit(BattleScapeState::BattleRunning)`: remove the per-run completion marker AND the
/// GTW-334 end-transition phase.
///
/// State-scoped-resource discipline (`bevy-traps.md` #1): [`BattleRunningComplete`] and the
/// GTW-334 [`EndTransition`] phase are per-run resources, so both are cleaned on exit — an
/// explicit-end re-entry starts ungated (no leaked marker) and unclassified (no leaked phase
/// from the prior battle's end). A `remove_resource` on an absent resource is a harmless no-op
/// (the marker is only ever inserted by the victory census / flee slices, and the phase only by
/// `move_on` once the marker is present).
pub(in crate::states::running::game::battlescape::battle_running) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<BattleRunningComplete>();
    commands.remove_resource::<EndTransition>();
}
