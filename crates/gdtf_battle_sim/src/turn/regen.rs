//! The pure turn-start TU-regeneration helper [`regen_team_tu`] (GTW-309).

use crate::{
    ganger::{Faction, Tu, TuMax},
    tu::reset_tu,
};

/// **Regenerate one team's TU at its turn-start** — reset [`Tu`] to [`TuMax`] for EVERY
/// ganger whose [`Faction`] matches `team`, leaving the other team untouched (GTW-309).
///
/// The turn-start budget refill is design canon (resolution.md §"What's pure math vs
/// sim": `reset_tu` is model-authoritative; stats.md: "A TU pool per turn"). This is the
/// per-team driver: it iterates the gangers and, for each on the newly-active `team`,
/// restores its pool to full via the landed [`reset_tu`] verb (which sets [`Tu`] =
/// [`TuMax`]). Gangers on any OTHER team are skipped, so the inactive team keeps whatever
/// TU it had — TU only regenerates for the team whose turn is starting.
///
/// A PLAIN function over an iterator of `(&Faction, &mut Tu, &TuMax)` (NOT a Bevy system),
/// so it is directly unit-testable from a borrowed component set without an `App` — the
/// [`dispatch_end_turn`](crate::turn::dispatch_end_turn) system feeds it the live query's
/// iterator. Render-free, no `World` access, no pixel.
pub fn regen_team_tu<'a, I>(gangers: I, team: Faction)
where
    I: IntoIterator<Item = (&'a Faction, &'a mut Tu, &'a TuMax)>,
{
    for (faction, tu, tu_max) in gangers {
        if *faction == team {
            reset_tu(tu, tu_max);
        }
    }
}
