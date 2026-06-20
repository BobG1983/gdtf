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

/// Inserts the [`BattleRunningComplete`] end-signal marker via [`Commands`].
///
/// The typed end-door a SIBLING battlescape module (the GTW-240 flee button in
/// `action_bar`) calls to end the live battle without naming [`BattleRunningComplete`]
/// directly. The resource lives in this private `resources` module — `support_item!`
/// declares it `pub(crate)` (`pub` under `test-support`), and the only re-export chain
/// that lifts it toward [`crate::test_support`] is `test-support`-gated — so it is NOT
/// nameable from `action_bar` in a production build. Rather than widen the resource's
/// visibility (which would collide with the existing `test-support` re-export of the same
/// name in this module's `mod.rs`), this small inserter is exported
/// `pub(in ...battlescape)` so the action-bar has ONE named door. Inserting the marker
/// trips the marker-gated [`move_on`](super::systems::move_on), advancing
/// `BattleRunning → AnimateOut` — flee becomes one new WRITER of the existing end signal
/// (the GTW-239 victory census is the other), NOT a new transition path.
///
/// Param-only (`bevy-traps.md` #7): takes `&mut Commands`, never `&mut World`. Idempotent
/// — a re-insert of the unit marker is harmless.
pub(in crate::states::running::game::battlescape) fn insert_battle_running_complete(
    commands: &mut Commands,
) {
    commands.insert_resource(BattleRunningComplete);
}
