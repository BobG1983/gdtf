use bevy::prelude::*;

use crate::scenes::running::game::battlescape::battle_running::resources::BattleRunningComplete;

/// `OnExit(BattleScapeState::BattleRunning)`: remove the per-run completion marker.
///
/// State-scoped-resource discipline (`bevy-traps.md` #1): [`BattleRunningComplete`] is a
/// per-run resource, so it is cleaned on exit — an explicit-end re-entry starts ungated (no
/// leaked marker). A `remove_resource` on an absent resource is a harmless no-op (the marker
/// is only ever inserted by the not-yet-wired victory/flee slices).
pub(in crate::scenes::running::game::battlescape::battle_running) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<BattleRunningComplete>();
}
