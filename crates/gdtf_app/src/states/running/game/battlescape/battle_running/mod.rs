mod plugin;
mod systems;
pub(in crate::states::running) use plugin::GameBattleScapeBattleRunningScenePlugin;
mod resources;
// The typed end-door (GTW-240): the sibling `action_bar` flee button calls this to insert
// the (private-module) `BattleRunningComplete` marker without naming the resource, so the
// resource stays encapsulated here and the existing `test-support` re-export below keeps
// its sole hold on the name. Re-exported UNCONDITIONALLY (it is real production wiring, not
// a test surface) at `pub(in ...battlescape)` so only the battlescape neighborhood reaches it.
pub(in crate::states::running::game::battlescape) use resources::insert_battle_running_complete;
/// Test-support re-exports for this scene (GTW-569 one-hop ledger): the explicit
/// end-signal marker (GTW-236) the reworked `state_walk` / `battle_running_driver` tests
/// insert through `crate::test_support` to stand in for victory/flee. The crate-root
/// ledger (`src/test_support.rs`) re-exports it by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::resources::BattleRunningComplete;
}
