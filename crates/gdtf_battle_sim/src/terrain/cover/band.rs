use bevy::prelude::Deref;

use crate::{cover::HeightBand, tuning::CombatTuning};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct BandFraction(f32);

impl BandFraction {
            #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }

                                                #[must_use]
    pub fn from_band(band: HeightBand, tuning: &CombatTuning) -> Self {
        let edges = &tuning.projectile_band_edges;
        match band {
            HeightBand::Low => Self(*edges.low_mid * 0.5),
            HeightBand::Mid => Self(*edges.low_mid),
            HeightBand::High => Self(*edges.mid_high),
        }
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BandRank(u8);

impl BandRank {
        #[must_use]
    pub const fn new(rung: u8) -> Self {
        Self(rung)
    }

                #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

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
