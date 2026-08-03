//! Stance contribution and brace engagement checks.

use bevy::prelude::Deref;

use crate::{
    cover::{BandRank, CoverEntry, HeightBand},
    ganger::StanceKind,
    stability::TerrainBraced,
    tuning::{ConeStabilityTuning, StanceContribution},
    weapon::Stable,
};

/// Whether brace is currently engaged.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BraceEngaged(bool);

impl BraceEngaged {
    pub(super) const fn new(engaged: bool) -> Self {
        Self(engaged)
    }
}

const fn band_rank(band: HeightBand) -> BandRank {
    match band {
        HeightBand::Low => BandRank::new(0),
        HeightBand::Mid => BandRank::new(1),
        HeightBand::High => BandRank::new(2),
    }
}

const fn brace_min_band(stance: StanceKind, tuning: &ConeStabilityTuning) -> HeightBand {
    match stance {
        StanceKind::Prone => tuning.brace_min_height.prone,
        StanceKind::Crouching => tuning.brace_min_height.kneel,
        StanceKind::Standing => tuning.brace_min_height.stand,
    }
}

/// Stability points contributed by the current stance.
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

/// Whether the shooter is braced (weapon stable, terrain braced, or cover high enough).
pub(super) fn brace_engages(
    stable: Stable,
    terrain_braced: TerrainBraced,
    stance: StanceKind,
    faced: Option<&CoverEntry>,
    tuning: &ConeStabilityTuning,
) -> BraceEngaged {
    if *stable || *terrain_braced {
        return BraceEngaged::new(true);
    }
    let Some(entry) = faced else {
        return BraceEngaged::new(false);
    };
    BraceEngaged::new(band_rank(entry.height_band) >= band_rank(brace_min_band(stance, tuning)))
}
