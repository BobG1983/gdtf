//! The §1a **brace gate + per-stance contributions** — the band-ordering, the
//! per-stance minimum brace band, the per-stance stability points, and whether
//! the automatic brace engages for a given weapon / stance / faced cover.

use bevy::prelude::Deref;

use crate::{
    cover::{BandRank, CoverEntry, HeightBand},
    ganger::StanceKind,
    stability::TerrainBraced,
    tuning::{ConeStabilityTuning, StanceContribution},
    weapon::Stable,
};

/// Whether the automatic **brace** engages for a shot (resolution.md §1a) — the OR of
/// the three unconditional brace sources (a `stable` weapon, a terrain-braced stair
/// kneel, or faced cover reaching the stance's minimum brace band).
///
/// A named verdict (no-bare-types) rather than a bare `bool`, so a caller feeding it
/// the single `tuning.brace_contribution` addend can never invert the sense of the
/// gate by accident. Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BraceEngaged(bool);

impl BraceEngaged {
    /// Build a brace-gate verdict from whether the brace engages.
    pub(super) const fn new(engaged: bool) -> Self {
        Self(engaged)
    }
}

/// The rank of a [`HeightBand`] on the LOW < MID < HIGH ladder — the orderable
/// view of the cover-height bands the brace gate compares.
///
/// [`HeightBand`] is a plain three-variant enum (no `Ord`), so the gate needs an
/// ordering: this maps each band to its position so "the faced band reaches the
/// stance's minimum band" is a `>=` over the ranks. It is **not** a hardcoded
/// band number — the *thresholds* still come from [`crate::tuning::BraceMinHeight`];
/// this only orders the enum the doc already orders ("LOW+ / MID+ / HIGH"). Returns
/// the shared [`BandRank`](crate::cover::BandRank) ordinal (no-bare-types).
const fn band_rank(band: HeightBand) -> BandRank {
    match band {
        HeightBand::Low => BandRank::new(0),
        HeightBand::Mid => BandRank::new(1),
        HeightBand::High => BandRank::new(2),
    }
}

/// The per-stance **minimum brace band** the faced cover must reach for this
/// stance, read off the tuning gate (resolution.md §1a: prone↔LOW+, kneel↔MID+,
/// stand↔HIGH). No band literal lives here — the threshold band is taken from
/// `tuning.brace_min_height`.
const fn brace_min_band(stance: StanceKind, tuning: &ConeStabilityTuning) -> HeightBand {
    match stance {
        StanceKind::Prone => tuning.brace_min_height.prone,
        StanceKind::Crouching => tuning.brace_min_height.kneel,
        StanceKind::Standing => tuning.brace_min_height.stand,
    }
}

/// The per-stance **stability contribution** for this stance, read off the tuning
/// triple (resolution.md §1a: prone 40 / kneel 25 / stand 10). No magnitude lives
/// here — the points are taken from `tuning.stance_stability`. The return states
/// its domain meaning in the type ([`StanceContribution`]), per `no-bare-types`.
pub(super) const fn stance_contribution(
    stance: StanceKind,
    tuning: &ConeStabilityTuning,
) -> StanceContribution {
    match stance {
        StanceKind::Prone => tuning.stance_stability.prone,
        StanceKind::Crouching => tuning.stance_stability.kneel,
        StanceKind::Standing => tuning.stance_stability.stand,
    }
}

/// Whether the **automatic brace** engages for `stance` against the `faced` cell,
/// given a weapon carrying the `stable` tag **or** a [`TerrainBraced`] stair brace —
/// `true` when **the weapon is `stable`** (a stable weapon braces UNCONDITIONALLY,
/// regardless of faced cover or stance — bipod-mounted / braced-by-design) **OR** the
/// shooter is **terrain-braced** (kneeling on a lower-endpoint stair cell under an
/// intact slab, GTW-392 — engages equally unconditionally) **OR** there is cover in
/// the faced cell whose [`HeightBand`], read **directly** off the [`CoverEntry`],
/// reaches the stance's minimum brace band from tuning (resolution.md §1a brace gate).
///
/// The three brace sources are **OR-combined** into a single `bool` and feed the SAME
/// single `tuning.brace_contribution` addend in [`crate::shot_pipeline::stability::score`]
/// — they can NEVER stack. A non-stable, non-terrain-braced weapon facing no cover
/// (`None`) never braces; [`crate::cover::band_for`] is not consulted.
pub(super) fn brace_engages(
    stable: Stable,
    terrain_braced: TerrainBraced,
    stance: StanceKind,
    faced: Option<&CoverEntry>,
    tuning: &ConeStabilityTuning,
) -> BraceEngaged {
    if *stable || *terrain_braced {
        // A stable weapon OR a terrain-braced stair kneel engages the brace
        // unconditionally — no cover / stance gate needed. The || is OR-combined with
        // the cover-height gate below so all three sources share one bool (never a sum).
        return BraceEngaged::new(true);
    }
    let Some(entry) = faced else {
        return BraceEngaged::new(false);
    };
    BraceEngaged::new(band_rank(entry.height_band) >= band_rank(brace_min_band(stance, tuning)))
}
