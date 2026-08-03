//! Weights for deriving ganger combat stats from attributes.

use bevy::{
    prelude::{Deref, Resource},
    reflect::TypePath,
};
use serde::Deserialize;

/// One attribute weight in a derived-stat formula.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StatWeight(f32);

impl StatWeight {
    /// Wrap a weight.
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

/// Wounds-per-HP divisor (design ≈10).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct WoundsPerHp(f32);

impl WoundsPerHp {
    /// Wrap a divisor.
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

/// Bottle-per-morale divisor (design ≈10).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BottlePerMorale(f32);

impl BottlePerMorale {
    /// Wrap a divisor.
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

/// Base TU before the speed slope (default 30).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuBase(f32);

impl TuBase {
    /// Wrap a base value.
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

/// TU added per point of speed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct TuPerSpeed(f32);

impl TuPerSpeed {
    /// Wrap a slope.
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

/// Weights for the shooting derived stat.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct ShootingWeights {
    /// Aim contribution.
    pub aim: StatWeight,
    /// Reflexes contribution.
    pub reflexes: StatWeight,
    /// Cool contribution.
    pub cool: StatWeight,
}

/// Weights for the fight derived stat.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct FightWeights {
    /// Speed contribution.
    pub speed: StatWeight,
    /// Strength contribution.
    pub strength: StatWeight,
    /// Grit contribution.
    pub grit: StatWeight,
    /// Cool contribution.
    pub cool: StatWeight,
}

/// Weights for the reactions derived stat.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct ReactionsWeights {
    /// Speed contribution.
    pub speed: StatWeight,
    /// Reflexes contribution.
    pub reflexes: StatWeight,
    /// Cool contribution.
    pub cool: StatWeight,
}

/// Weights for HP (cool defaults to 0.5).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HpWeights {
    /// Grit contribution.
    pub grit: StatWeight,
    /// Toughness contribution.
    pub toughness: StatWeight,
    /// Cool contribution.
    pub cool: StatWeight,
}

impl Default for HpWeights {
    fn default() -> Self {
        Self {
            grit: StatWeight::default(),
            toughness: StatWeight::default(),
            cool: StatWeight::new(0.5),
        }
    }
}

/// Weights for morale.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct MoraleWeights {
    /// Grit contribution.
    pub grit: StatWeight,
    /// Cool contribution.
    pub cool: StatWeight,
}

/// Full ganger stat derivation resource.
#[derive(Debug, Clone, PartialEq, Default, Resource, Deserialize, TypePath)]
pub struct GangerStatTuning {
    /// Shooting weights.
    pub shooting: ShootingWeights,
    /// Fight weights.
    pub fight: FightWeights,
    /// Reactions weights.
    pub reactions: ReactionsWeights,
    /// HP weights.
    pub hp: HpWeights,
    /// Morale weights.
    pub morale: MoraleWeights,
    /// Wounds per HP.
    pub wounds_per_hp: WoundsPerHp,
    /// Bottle per morale.
    pub bottle_per_morale: BottlePerMorale,
    /// TU base.
    pub tu_base: TuBase,
    /// TU per speed.
    pub tu_per_speed: TuPerSpeed,
}
