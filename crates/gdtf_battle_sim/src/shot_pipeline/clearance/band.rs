use crate::{
    cover::{BandFraction, BandRank, HeightBand, band_for},
    ganger::StanceKind,
    metric::{SimPos, pos_to_cell},
    tuning::CombatTuning,
};

#[must_use]
pub const fn silhouette_band(stance: StanceKind) -> HeightBand {
    match stance {
        StanceKind::Standing => HeightBand::High,
        StanceKind::Crouching => HeightBand::Mid,
        StanceKind::Prone => HeightBand::Low,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clearance {
            Clears,
            Impacts,
}

pub(super) const fn band_rank(band: HeightBand) -> BandRank {
    match band {
        HeightBand::Low => BandRank::new(0),
        HeightBand::Mid => BandRank::new(1),
        HeightBand::High => BandRank::new(2),
    }
}

#[must_use]
pub fn round_band_fraction(round: SimPos) -> BandFraction {
    let (_, level) = pos_to_cell(round);
    let above_floor = round.z - f32::from(*level);
    BandFraction::new(above_floor)
}

#[must_use]
pub fn round_band_for_cell(round: SimPos, tuning: &CombatTuning) -> HeightBand {
    band_for(round_band_fraction(round), tuning)
}

#[must_use]
pub const fn lower_band(a: HeightBand, b: HeightBand) -> HeightBand {
    if band_rank(a).get() <= band_rank(b).get() {
        a
    } else {
        b
    }
}

#[must_use]
pub const fn round_clears_occupant(round_band: HeightBand, occupant_band: HeightBand) -> Clearance {
    if band_rank(round_band).get() > band_rank(occupant_band).get() {
        Clearance::Clears
    } else {
        Clearance::Impacts
    }
}
