use bevy::prelude::*;

crate::support_item! {
    /// The explicit **end-signal marker** for the `BattleRunning` phase.
    ///
    /// A unit marker `Resource` (it carries no domain value, so no newtype): its mere
    /// PRESENCE is the signal. While it is absent the battlescape RESTS in
    /// [`BattleScapeState::BattleRunning`](crate::states::BattleScapeState::BattleRunning)
    /// indefinitely; once a slice INSERTS it, the marker-gated `move_on` advances
    /// `BattleRunning → AnimateOut`. The victory census and the flee button are the slices
    /// that insert it; this lifecycle slice (GTW-236) leaves it un-inserted, so the battle
    /// persists until one of those lands.
    ///
    /// It is a per-run state-scoped resource (`bevy-traps.md` #1): never inserted
    /// `OnEnter(BattleRunning)`, and removed `OnExit(BattleRunning)` by `cleanup` so an
    /// explicit-end re-entry starts ungated (no leaked marker).
    ///
    /// `support_item!`-flipped to `pub` under the `test-support` feature so the reworked
    /// `state_walk` / `battle_running_driver` integration tests can insert it through the
    /// `crate::test_support` surface to stand in for the not-yet-wired victory/flee end
    /// condition; `pub(crate)` otherwise.
    #[derive(Resource)]
    struct BattleRunningComplete;
}
