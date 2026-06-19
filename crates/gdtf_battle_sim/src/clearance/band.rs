//! The clearance-banding implementation — [`Clearance`], the band-rank ladder, and the
//! round-band / clearance predicates. See the module docs (`super`) for the §2 rule.

use crate::{
    cover::{BandFraction, HeightBand, band_for},
    ganger::StanceKind,
    metric::{SimPos, pos_to_cell},
    tuning::CombatTuning,
};

/// The **silhouette band** a ganger presents to the march while holding `stance` —
/// the band a round must fly strictly higher than to clear the ganger
/// (`docs/combat/resolution.md` §1: stance "reshapes the clearance silhouette";
/// §4: "standing (full height), kneeling (compressed, legs tucked low), prone (very
/// low/flat)"; `docs/combat/battle-space.md` §"Banding": the LOW/MID/HIGH band "pairs
/// with stance (prone / kneel / stand)").
///
/// The canonical stance → band mapping, pinned in one place so the change-driven
/// maintenance can publish the occupant's band off its [`Stance`](crate::ganger::Stance)
/// component and the march reads it back via
/// [`OccupancyGrid::occupant_band`](crate::occupancy::OccupancyGrid::occupant_band):
///
/// - [`StanceKind::Standing`] → [`HeightBand::High`] — full / high silhouette; the
///   doc's "stand" (only a HIGH round flies strictly over it within a storey).
/// - [`StanceKind::Crouching`] → [`HeightBand::Mid`] — compressed silhouette; the
///   doc's "kneel".
/// - [`StanceKind::Prone`] → [`HeightBand::Low`] — lowest / flat silhouette; the
///   doc's "prone".
///
/// This is the silhouette band (what a round must clear), distinct from the
/// per-stance *muzzle*/aim heights ([`crate::central_axis`]'s `SilhouetteTops`): the
/// band is the coarse three-rung clearance abstraction the march compares, not a
/// continuous level-fraction.
#[must_use]
pub const fn silhouette_band(stance: StanceKind) -> HeightBand {
    match stance {
        StanceKind::Standing => HeightBand::High,
        StanceKind::Crouching => HeightBand::Mid,
        StanceKind::Prone => HeightBand::Low,
    }
}

/// The outcome of the per-crossing clearance test — whether the round sails over
/// the occupant or impacts it (`docs/combat/resolution.md` §2:
/// "strictly higher sails over; equal-or-lower impacts").
///
/// A named domain enum (no-bare-types: a clearance verdict is a domain value, not
/// a bare `bool`) so a caller can never invert the sense by accident. The march
/// continues past a [`Clearance::Clears`] occupant and stops on a
/// [`Clearance::Impacts`] one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clearance {
    /// The round flies **strictly higher** than the occupant's band — it sails over
    /// and the march continues (no impact at this cell).
    Clears,
    /// The round is **equal-or-lower** than the occupant's band — it impacts the
    /// occupant and the march stops here.
    Impacts,
}

/// The rank of a [`HeightBand`] on the LOW < MID < HIGH ladder — the orderable view
/// of the bands the clearance test compares.
///
/// [`HeightBand`] is a plain three-variant enum (no `Ord`), so "strictly higher"
/// needs an ordering: this maps each band to its rung so the predicate is a `>`
/// over the ranks. It is **not** a band threshold — the band *edges* still live in
/// [`crate::tuning::ProjectileBandEdges`] and reach a band only through
/// [`band_for`]; this merely orders the enum the docs already order
/// (resolution.md §2's LOW/MID/HIGH ladder).
pub(super) const fn band_rank(band: HeightBand) -> u8 {
    match band {
        HeightBand::Low => 0,
        HeightBand::Mid => 1,
        HeightBand::High => 2,
    }
}

/// The round's **within-level [`BandFraction`]** at the cell it is crossing — its
/// continuous `z` height **above that cell's level floor**, in sim units
/// (`docs/combat/resolution.md` §2; `docs/combat/battle-space.md` §"Banding").
///
/// The storey index `k` is the one [`pos_to_cell`] floors `round` into; the fraction
/// is `round.z − k` — the part of `z` that lies *within* that level (AC #1, #5). On a
/// well-formed crossing this lands in `[0, 1)`; a degenerate / below-floor `z` (`z <
/// k`) yields a **negative** fraction, which [`band_for`] classifies as
/// [`HeightBand::Low`] (it is strictly below the LOW→MID edge) — graceful, no panic
/// (AC #5). The storey floor offset is applied *per level*, so the same above-floor
/// fraction classifies the same band on every storey.
#[must_use]
pub fn round_band_fraction(round: SimPos) -> BandFraction {
    let (_, level) = pos_to_cell(round);
    // The within-level fraction: continuous z minus the crossed cell's storey floor.
    // f32::from(u8) is exact for the 0..MAX_LEVELS storey range.
    let above_floor = round.z - f32::from(*level);
    BandFraction::new(above_floor)
}

/// The round's [`HeightBand`] at the cell it is crossing — its
/// [`round_band_fraction`] classified by the landed E1 [`band_for`] against the
/// tunable [`crate::tuning::ProjectileBandEdges`] edges (`docs/combat/resolution.md`
/// §2; AC #1).
///
/// Band classification is **reused** from E1 (`band_for`), never reimplemented (AC
/// #6): the band edges come solely from `tuning.projectile_band_edges`, so a tuning
/// edit moves the boundaries (AC #4). The per-level floor offset is handled by
/// [`round_band_fraction`], so a round at the same above-floor fraction classifies
/// the same band on every storey. A below-floor / degenerate fraction classifies
/// [`HeightBand::Low`] gracefully (AC #5).
#[must_use]
pub fn round_band_for_cell(round: SimPos, tuning: &CombatTuning) -> HeightBand {
    band_for(round_band_fraction(round), tuning)
}

/// The per-crossing **clearance predicate**: does a round in `round_band` clear an
/// occupant in `occupant_band`? (`docs/combat/resolution.md` §2:
/// "strictly higher sails over; equal-or-lower impacts"; AC #2, #3.)
///
/// **Strictly higher** (the round's band ranks above the occupant's) ⇒
/// [`Clearance::Clears`]; **equal-or-lower** ⇒ [`Clearance::Impacts`]. This is the
/// whole rule — no exemption list (the retired aim-occlusion special-casing,
/// resolution.md §2). It reproduces the prone-can't-clear-LOW consequence purely by
/// band: a LOW round versus a LOW occupant is *equal*, so it impacts (AC #3).
#[must_use]
pub const fn round_clears_occupant(round_band: HeightBand, occupant_band: HeightBand) -> Clearance {
    if band_rank(round_band) > band_rank(occupant_band) {
        Clearance::Clears
    } else {
        Clearance::Impacts
    }
}
