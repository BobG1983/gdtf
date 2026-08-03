//! `#[serde(transparent)]` so it round-trips as a bare RON scalar. The derivation
use bevy::{
    prelude::{Deref, Resource},
    reflect::TypePath,
};
use serde::Deserialize;

/// `#[serde(transparent)]` lets an authored weight parse as a bare scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StatWeight(f32);

impl StatWeight {
            #[must_use]
    pub const fn new(weight: f32) -> Self {
        Self(weight)
    }
}

impl Default for StatWeight {
        fn default() -> Self {
        Self(1.0)
    }
}

/// stats.md ≈10). `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct WoundsPerHp(f32);

impl WoundsPerHp {
        #[must_use]
    pub const fn new(divisor: f32) -> Self {
        Self(divisor)
    }
}

impl Default for WoundsPerHp {
        fn default() -> Self {
        Self(10.0)
    }
}

/// stats.md ≈10). `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BottlePerMorale(f32);

impl BottlePerMorale {
        #[must_use]
    pub const fn new(divisor: f32) -> Self {
        Self(divisor)
    }
}

impl Default for BottlePerMorale {
        fn default() -> Self {
        Self(10.0)
    }
}

/// derivation coefficient is `f32` then rounded). Defaults to `30.0`. `#[serde(transparent)]`
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuBase(f32);

impl TuBase {
        #[must_use]
    pub const fn new(base: f32) -> Self {
        Self(base)
    }
}

impl Default for TuBase {
            fn default() -> Self {
        Self(30.0)
    }
}

/// `#[serde(transparent)]` lets it parse as a bare scalar. Consumed by
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuPerSpeed(f32);

impl TuPerSpeed {
        #[must_use]
    pub const fn new(slope: f32) -> Self {
        Self(slope)
    }
}

impl Default for TuPerSpeed {
        fn default() -> Self {
        Self(10.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ShootingWeights {
        pub aim:      StatWeight,
        pub reflexes: StatWeight,
        pub cool:     StatWeight,
}

impl Default for ShootingWeights {
        fn default() -> Self {
        Self {
            aim:      StatWeight::default(),
            reflexes: StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FightWeights {
        pub speed:    StatWeight,
        pub strength: StatWeight,
        pub grit:     StatWeight,
        pub cool:     StatWeight,
}

impl Default for FightWeights {
        fn default() -> Self {
        Self {
            speed:    StatWeight::default(),
            strength: StatWeight::default(),
            grit:     StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ReactionsWeights {
        pub speed:    StatWeight,
        pub reflexes: StatWeight,
        pub cool:     StatWeight,
}

impl Default for ReactionsWeights {
        fn default() -> Self {
        Self {
            speed:    StatWeight::default(),
            reflexes: StatWeight::default(),
            cool:     StatWeight::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HpWeights {
        pub grit:      StatWeight,
        pub toughness: StatWeight,
        pub cool:      StatWeight,
}

impl Default for HpWeights {
        fn default() -> Self {
        Self {
            grit:      StatWeight::default(),
            toughness: StatWeight::default(),
            cool:      StatWeight::new(0.5),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MoraleWeights {
        pub grit: StatWeight,
        pub cool: StatWeight,
}

impl Default for MoraleWeights {
        fn default() -> Self {
        Self {
            grit: StatWeight::default(),
            cool: StatWeight::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct GangerStatTuning {
        pub shooting:          ShootingWeights,
        pub fight:             FightWeights,
        pub reactions:         ReactionsWeights,
        pub hp:                HpWeights,
        pub morale:            MoraleWeights,
        pub wounds_per_hp:     WoundsPerHp,
        pub bottle_per_morale: BottlePerMorale,
        pub tu_base:           TuBase,
        pub tu_per_speed:      TuPerSpeed,
}
