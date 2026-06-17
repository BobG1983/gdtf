//! The §1a **brace gate + per-stance contributions** — the band-ordering, the
//! per-stance minimum brace band, the per-stance stability points, and whether
//! the automatic brace engages for a given weapon / stance / faced cover.

use crate::{
    cover::{CoverEntry, HeightBand},
    ganger::StanceKind,
    tuning::{ConeStabilityTuning, StanceContribution},
    weapon::Stable,
};

/// The rank of a [`HeightBand`] on the LOW < MID < HIGH ladder — the orderable
/// view of the cover-height bands the brace gate compares.
///
/// [`HeightBand`] is a plain three-variant enum (no `Ord`), so the gate needs an
/// ordering: this maps each band to its position so "the faced band reaches the
/// stance's minimum band" is a `>=` over the ranks. It is **not** a hardcoded
/// band number — the *thresholds* still come from [`crate::tuning::BraceMinHeight`];
/// this only orders the enum the doc already orders ("LOW+ / MID+ / HIGH").
const fn band_rank(band: HeightBand) -> u8 {
    match band {
        HeightBand::Low => 0,
        HeightBand::Mid => 1,
        HeightBand::High => 2,
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

/// Whether the **automatic brace** engages for `stance` against the `faced` cell
/// with a weapon carrying the `stable` tag — `true` when **the weapon is `stable`**
/// (a stable weapon braces UNCONDITIONALLY, regardless of faced cover or stance —
/// bipod-mounted / braced-by-design) **OR** there is cover in the faced cell whose
/// [`HeightBand`], read **directly** off the [`CoverEntry`], reaches the stance's
/// minimum brace band from tuning (resolution.md §1a brace gate). A non-stable
/// weapon facing no cover (`None`) never braces; [`crate::cover::band_for`] is not
/// consulted.
pub(super) fn brace_engages(
    stable: Stable,
    stance: StanceKind,
    faced: Option<&CoverEntry>,
    tuning: &ConeStabilityTuning,
) -> bool {
    if *stable {
        // A stable weapon engages the brace unconditionally — no cover / stance gate.
        return true;
    }
    let Some(entry) = faced else {
        return false;
    };
    band_rank(entry.height_band) >= band_rank(brace_min_band(stance, tuning))
}
