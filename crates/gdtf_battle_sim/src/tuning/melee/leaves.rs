use bevy::prelude::Deref;
use serde::Deserialize;


/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeKMargin(f32);

impl MeleeKMargin {
                                #[must_use]
    pub const fn new(k_margin: f32) -> Self {
        Self(k_margin)
    }
}

impl Default for MeleeKMargin {
    fn default() -> Self {
        Self(1.0)
    }
}

/// and the clamp edges hold, never this magnitude. `#[serde(transparent)]` lets
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMin(f32);

impl MeleeMultMin {
                        #[must_use]
    pub const fn new(mult_min: f32) -> Self {
        Self(mult_min)
    }
}

impl Default for MeleeMultMin {
    fn default() -> Self {
        Self(0.5)
    }
}

/// `#[serde(transparent)]` lets it parse a bare RON scalar; private inner +
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MeleeMultMax(f32);

impl MeleeMultMax {
                        #[must_use]
    pub const fn new(mult_max: f32) -> Self {
        Self(mult_max)
    }
}

impl Default for MeleeMultMax {
    fn default() -> Self {
        Self(3.0)
    }
}

/// distribution-shape invariants, never this magnitude. `#[serde(transparent)]`
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FightVariance(f32);

impl FightVariance {
                        #[must_use]
    pub const fn new(variance: f32) -> Self {
        Self(variance)
    }
}

impl Default for FightVariance {
    fn default() -> Self {
        Self(0.2)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct MeleeTuning {
                pub k_margin: MeleeKMargin,
            pub mult_min: MeleeMultMin,
            pub mult_max: MeleeMultMax,
            pub variance: FightVariance,
}
