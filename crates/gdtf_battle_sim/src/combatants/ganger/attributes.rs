//! Authored primary attributes (speed, aim, strength, etc.).

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::ganger::vitals::{Luck, Toughness};

/// Movement attribute.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Speed(f32);

impl Speed {
    /// Wrap a speed value.
    #[must_use]
    pub const fn new(speed: f32) -> Self {
        Self(speed)
    }
}

/// Ranged accuracy attribute.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Aim(f32);

impl Aim {
    /// Wrap an aim value.
    #[must_use]
    pub const fn new(aim: f32) -> Self {
        Self(aim)
    }
}

/// Melee power attribute.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strength(f32);

impl Strength {
    /// Wrap a strength value.
    #[must_use]
    pub const fn new(strength: f32) -> Self {
        Self(strength)
    }
}

/// Reaction speed attribute.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reflexes(f32);

impl Reflexes {
    /// Wrap a reflexes value.
    #[must_use]
    pub const fn new(reflexes: f32) -> Self {
        Self(reflexes)
    }
}

/// Composure under fire.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cool(f32);

impl Cool {
    /// Wrap a cool value.
    #[must_use]
    pub const fn new(cool: f32) -> Self {
        Self(cool)
    }
}

/// Resilience / toughness contribution.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grit(f32);

impl Grit {
    /// Wrap a grit value.
    #[must_use]
    pub const fn new(grit: f32) -> Self {
        Self(grit)
    }
}

/// Bundle of all eight authored attributes (toughness and luck live in vitals).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GangerAttributes {
    /// Speed.
    pub speed:     Speed,
    /// Aim.
    pub aim:       Aim,
    /// Strength.
    pub strength:  Strength,
    /// Toughness.
    pub toughness: Toughness,
    /// Reflexes.
    pub reflexes:  Reflexes,
    /// Cool.
    pub cool:      Cool,
    /// Grit.
    pub grit:      Grit,
    /// Luck.
    pub luck:      Luck,
}
