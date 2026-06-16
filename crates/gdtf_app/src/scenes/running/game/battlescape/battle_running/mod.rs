mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameBattleScapeBattleRunningScenePlugin;
mod resources;
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean (the action-bar marker re-export chain
// precedent). Carries `BattleRunningComplete` up toward `crate::test_support` so the reworked
// `state_walk` / `battle_running_driver` tests can insert it to stand in for victory/flee.
#[cfg(feature = "test-support")]
crate::support_use!(resources::BattleRunningComplete;);
