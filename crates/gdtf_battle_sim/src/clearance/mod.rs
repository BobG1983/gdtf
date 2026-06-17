//! Per-crossing **clearance banding** — the march's "does the round clear this
//! occupant?" test, expressed purely as band-vs-band (`docs/combat/resolution.md`
//! §2; `docs/combat/battle-space.md` §"Banding" + the clearance paragraph).
//!
//! This is the E2.6 slice. The coarse march ([`crate::march`], E2.7) flies a
//! round through the grid and, at **each occupied cell it crosses**, must decide
//! whether the round sails over the occupant or impacts it. Per resolution.md §2
//! that decision is **strictly-higher SAILS OVER / equal-or-lower IMPACTS**, with
//! the round and the occupant each reduced to a
//! [`HeightBand`](crate::cover::HeightBand) — there is no special-case exemption
//! list (the old aim-occlusion exceptions are retired: resolution.md §2 /
//! battle-space.md §"Banding"). This module is the **one** clearance truth E2.7's
//! march and E3 share, so the rule is pinned independently.
//!
//! Two thin pieces, both in the cubic-voxel metric ([`SimPos`](crate::metric::SimPos),
//! one cell = one level = 1.0 sim unit) and **zero pixels**:
//!
//! 1. [`round_band_for_cell`] — the round's [`HeightBand`](crate::cover::HeightBand) at a
//!    crossed cell. The round's height above the crossed cell's level floor is a
//!    **within-level fraction** in sim units: `fraction = pos.z − k` where `k` is the
//!    storey index [`pos_to_cell`](crate::metric::pos_to_cell) floors the round into
//!    (resolution.md §2 / AC #1, #5). That fraction, wrapped as a
//!    [`BandFraction`](crate::cover::BandFraction), is classified by the landed E1
//!    [`band_for`](crate::cover::band_for) against the tunable
//!    [`crate::tuning::ProjectileBandEdges`] level-fraction edges — band classification
//!    is **reused**, never reimplemented (AC #6). A below-floor / degenerate fraction
//!    (`z < k`) is negative and classifies [`HeightBand::Low`](crate::cover::HeightBand::Low)
//!    gracefully — no panic (AC #5).
//! 2. [`round_clears_occupant`] — the named clearance predicate: the round's band
//!    **strictly above** the occupant's band ⇒ [`Clearance::Clears`] (sails over,
//!    no impact); **equal-or-lower** ⇒ [`Clearance::Impacts`] (resolution.md §2 /
//!    AC #2). This reproduces "a prone shooter can't clear even LOW cover" by band
//!    alone — a LOW round vs a LOW occupant is *equal*, so it impacts (AC #3) — with
//!    no exemption list.

mod band;
#[cfg(test)]
mod test;

pub use band::{Clearance, round_band_for_cell, round_band_fraction, round_clears_occupant};
