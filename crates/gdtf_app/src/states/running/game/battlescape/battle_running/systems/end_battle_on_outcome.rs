//! [`end_battle_on_outcome`] — the thin app-side glue that ends `BattleRunning` on the
//! sim's outcome signal (GTW-239).
//!
//! When the sim declares the battle decided — its
//! [`BattleWon`](gdtf_battle_sim::BattleWon) OR [`BattleLost`](gdtf_battle_sim::BattleLost)
//! message, emitted by the `sim-victory-census` slice's `check_outcome` (GTW-237) — this
//! system inserts the EXISTING [`BattleRunningComplete`] marker, so the marker-gated
//! [`move_on`](super::move_on) advances `BattleRunning → AnimateOut → AfterMath`. It is the
//! outcome twin of `gate_generation_complete` (GTW-207): a `MessageReader<sim signal>`
//! `.after(SimSystems::Simulate)`, presence-gated, that flips an app-owned completion marker.
//!
//! It deliberately owns NO combat / win / loss rule (the roster-grounded census + the
//! `BattleWon`/`BattleLost` emit are the sim's, GTW-237); it does NOT add the outcome message
//! buffers (the sim's `BattleSimPlugin` owns those `add_message`s); and it does NOT change the
//! `move_on` / `AnimateOut` / `AfterMath` chain. WIN and LOSS both end the battle via the SAME
//! chain — the aftermath does not yet branch on win-vs-loss (a future slice).

use bevy::prelude::*;

use crate::states::running::game::battlescape::battle_running::resources::BattleRunningComplete;

/// `Update` (presence-gated, `.after(SimSystems::Simulate)`): end `BattleRunning` on EITHER
/// sim outcome signal by inserting [`BattleRunningComplete`].
///
/// Reads BOTH sim-owned outcome buffers — [`BattleWon`](gdtf_battle_sim::BattleWon) and
/// [`BattleLost`](gdtf_battle_sim::BattleLost) — and, if EITHER carried a message this run,
/// inserts the [`BattleRunningComplete`] marker via [`Commands`]. The marker-gated
/// [`move_on`](super::move_on) then sets `NextState(AnimateOut)` (unchanged by this slice).
///
/// BOTH readers are drained every run (via a non-short-circuiting `|`, NOT `||`) so neither
/// buffer backs up — a short-circuiting `||` would leave the second reader un-drained when the
/// first already had a message. The plugin gate
/// (`in_state(BattleRunning).and(not(resource_exists::<BattleRunningComplete>))`) plus the
/// idempotent [`insert_resource`](Commands::insert_resource) make a repeated / multi-frame
/// outcome (a census that re-declares the outcome each tick) a no-op — the marker is inserted
/// at most once, so the machine advances out of `BattleRunning` exactly once.
///
/// No `&mut World` (`bevy-traps.md` #7): the work is two [`MessageReader`]s + [`Commands`],
/// both panic-free when the buffers are empty, so the system is safe whether or not an outcome
/// ever arrives.
pub(in crate::states::running::game::battlescape::battle_running) fn end_battle_on_outcome(
    mut won: MessageReader<gdtf_battle_sim::BattleWon>,
    mut lost: MessageReader<gdtf_battle_sim::BattleLost>,
    mut commands: Commands,
) {
    // Drain BOTH readers (non-short-circuiting `|`) so neither buffer backs up, and end on
    // either outcome. WIN and LOSS both end the battle via the same chain.
    let decided = won.read().next().is_some() | lost.read().next().is_some();
    if decided {
        // Idempotent: the `not(resource_exists::<…>)` gate makes repeats no-ops, and a
        // re-insert of the unit marker is harmless either way.
        commands.insert_resource(BattleRunningComplete);
    }
}
