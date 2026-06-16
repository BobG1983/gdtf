mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::GameBattleScapeBattleRunningScenePlugin;
mod resources;
// The typed end-door (GTW-240): the sibling `action_bar` flee button calls this to insert
// the (private-module) `BattleRunningComplete` marker without naming the resource, so the
// resource stays encapsulated here and the existing `test-support` re-export below keeps
// its sole hold on the name. Re-exported UNCONDITIONALLY (it is real production wiring, not
// a test surface) at `pub(in ...battlescape)` so only the battlescape neighborhood reaches it.
pub(in crate::scenes::running::game::battlescape) use resources::insert_battle_running_complete;
// Test-support-only re-export of the explicit end-signal marker (GTW-236), gated so the
// binary build stays `unused`/`unreachable_pub`-clean (the action-bar marker re-export chain
// precedent). Carries `BattleRunningComplete` up toward `crate::test_support` so the reworked
// `state_walk` / `battle_running_driver` tests can insert it to stand in for victory/flee.
#[cfg(feature = "test-support")]
crate::support_use!(resources::BattleRunningComplete;);
