//! The input system-ordering anchor (GTW-245): the [`InputSystems`] band every battle
//! input system runs in, ordered `.before(SimSystems::Simulate)`.

use bevy::prelude::*;

/// Input system-ordering anchor — the named `Update`-schedule band every battle input
/// system runs in, configured `.before(`[`SimSystems::Simulate`](gdtf_battle_sim::occupancy_sync::SimSystems)`)`.
///
/// Mirrors the sim's `SimSystems::Simulate` and the presenter's `PresenterSystems::Draw`
/// anchors (the proven cross-crate ordering pattern): the band is defined ONCE via
/// [`configure_sets`](App::configure_sets), then `.in_set` on each member system
/// (`bevy-traps.md` #5 — `configure_sets` precedes `.in_set`). Putting the whole input
/// band `.before(SimSystems::Simulate)` realizes the documented one-way
/// `input -> sim -> presenter` loop (ADR-0001) inside a single `Update`: a click's
/// `*Requested` message is EMITTED before the sim consumes it the SAME frame, removing the
/// input/sim ordering ambiguity (`bevy-traps.md` #3 — no flaky one-frame lag). The ordering
/// edge is independent of the sim band's `run_if` gate (an edge is not a run condition), so
/// it is correct whether or not a battle is live.
///
/// A framework `SystemSet` label, not a domain value (the no-bare-types framework carve-out,
/// the same justification `SimSystems` / `PresenterSystems` use) — `pub` so the app and
/// tests can name it for ordering and probing.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSystems {
    /// The band holding every battle input system — ordered before the sim's world
    /// mutations so a `*Requested` message is emitted the same update the sim runs.
    Gather,
}
