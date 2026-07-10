//! Height-band classification — the within-level [`BandFraction`] datum and the
//! [`band_for`] classifier that maps it to a [`HeightBand`] using ONLY the tunable
//! [`BandEdge`](crate::tuning::BandEdge) level-fraction edges (no hardcoded band
//! numbers — C6, `docs/combat/battle-space.md` §"Banding").

use bevy::prelude::Deref;

use crate::{cover::HeightBand, tuning::CombatTuning};

/// A **within-level fraction** — a height above the crossed cell's level floor,
/// expressed as a dimensionless fraction of one level's height (`z ∈ [0,1)`
/// within a storey). The input to band classification
/// (`docs/combat/battle-space.md` §"Banding": the continuous level-fraction
/// datums fed into the band assignment).
///
/// A named newtype over `f32` (no-bare-types): a within-level clearance fraction
/// is a domain value, distinct from a [`BandEdge`](crate::tuning::BandEdge)
/// *threshold* it is compared against. Used only as the [`band_for`] input. Private
/// inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct BandFraction(f32);

impl BandFraction {
    /// Build a within-level clearance fraction from its magnitude (a fraction of
    /// one level's height).
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }

    /// A representative within-level fraction for an already-classified
    /// [`HeightBand`], read from `tuning`'s edges — never a hardcoded band number
    /// (C6).
    ///
    /// Used by [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover)
    /// to round-trip a stored band through the tuning edges so the band can never
    /// drift from the current tuning: LOW maps below the LOW→MID edge, MID between
    /// the two edges, HIGH at or above the MID→HIGH edge. The exact fraction is
    /// immaterial — only that it lands back in the same band under [`band_for`] — so
    /// it is derived purely from `tuning`'s authored edges, with no literal magnitude
    /// beyond an arbitrary sub-edge nudge.
    #[must_use]
    pub fn from_band(band: HeightBand, tuning: &CombatTuning) -> Self {
        let edges = &tuning.projectile_band_edges;
        match band {
            // Strictly below the LOW→MID edge → LOW (half the edge stays below it
            // for any positive edge, with no literal threshold of our own).
            HeightBand::Low => Self(*edges.low_mid * 0.5),
            // At-or-above LOW→MID, strictly below MID→HIGH → MID.
            HeightBand::Mid => Self(*edges.low_mid),
            // At-or-above the MID→HIGH edge → HIGH.
            HeightBand::High => Self(*edges.mid_high),
        }
    }
}

/// The rank of a [`HeightBand`] on the LOW < MID < HIGH ladder — the orderable view
/// of the bands the clearance and brace gates compare.
///
/// [`HeightBand`] is a plain three-variant enum (deliberately no `Ord`), so any
/// "strictly higher" / "reaches this band" comparison needs an explicit ordering:
/// this newtype IS that ordering, so a rank comparison is a `>`/`>=` over the derived
/// [`Ord`]. It is **not** a band *threshold* — the band edges still live in
/// [`crate::tuning`] and reach a band only through [`band_for`]; this merely orders
/// the enum the docs already order (`docs/combat/resolution.md` §2's LOW/MID/HIGH
/// ladder). A named newtype over `u8` (no-bare-types: a band rank is a domain ordinal,
/// not a bare integer). Private inner + derived [`Deref`]; shared by the shot-pipeline
/// clearance test and the stability brace gate so the one ladder is pinned once.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BandRank(u8);

impl BandRank {
    /// Build a band rank from its rung on the LOW(0) < MID(1) < HIGH(2) ladder.
    #[must_use]
    pub const fn new(rung: u8) -> Self {
        Self(rung)
    }

    /// The rung on the ladder as its raw `u8` — a `const` reader so a `const fn`
    /// clearance predicate can compare two ranks (the derived [`Ord`] operators are
    /// not usable in a `const` context).
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Classify a within-level clearance fraction into a [`HeightBand`] using the
/// tunable [`BandEdge`](crate::tuning::BandEdge) level-fraction edges off
/// [`CombatTuning`] — **no hardcoded band numbers** (C6,
/// `docs/combat/battle-space.md` §"Banding"; `docs/combat/resolution.md` §2's
/// `band_for`).
///
/// A fraction **strictly below** the LOW→MID edge is [`HeightBand::Low`];
/// at-or-above it but **strictly below** the MID→HIGH edge is [`HeightBand::Mid`];
/// at-or-above the MID→HIGH edge is [`HeightBand::High`]. The two edges are read
/// from `tuning.projectile_band_edges` (the tunable level-fractions ≈ ⅓ and ⅔ of a
/// level) — this function never names a magnitude itself.
#[must_use]
pub fn band_for(fraction: BandFraction, tuning: &CombatTuning) -> HeightBand {
    let edges = &tuning.projectile_band_edges;
    if *fraction < *edges.low_mid {
        HeightBand::Low
    } else if *fraction < *edges.mid_high {
        HeightBand::Mid
    } else {
        HeightBand::High
    }
}
