//! Ganger stat derivation weights. Scalars use `#[serde(transparent)]` so RON is bare numbers.
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

/// Wounds-per-HP divisor (design ≈10). Bare scalar in RON.
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

/// Bottle-per-morale divisor (design ≈10). Bare scalar in RON.
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

/// Base TU before speed slope (default 30). Bare scalar in RON.
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

/// TU added per point of speed. Bare scalar in RON.
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct ShootingWeights {
    pub aim:      StatWeight,
    pub reflexes: StatWeight,
    pub cool:     StatWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct FightWeights {
    pub speed:    StatWeight,
    pub strength: StatWeight,
    pub grit:     StatWeight,
    pub cool:     StatWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct ReactionsWeights {
    pub speed:    StatWeight,
    pub reflexes: StatWeight,
    pub cool:     StatWeight,
}

/// HP weights. Cool defaults to 0.5 (not field Default), so this stays manual.
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
pub struct MoraleWeights {
    pub grit: StatWeight,
    pub cool: StatWeight,
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
