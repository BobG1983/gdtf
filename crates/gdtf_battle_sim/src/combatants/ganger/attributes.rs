//! (`docs/combat/stats.md` §"Direct attributes (8)"). These are AUTHORED on a
//! [`Deref`](bevy::prelude::Deref) (house style) and `#[serde(transparent)]` so an
use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

use crate::ganger::vitals::{Luck, Toughness};

/// derivation can query `&Speed` alone. Defaults to `0.0`. `#[serde(transparent)]`
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Speed(f32);

impl Speed {
        #[must_use]
    pub const fn new(speed: f32) -> Self {
        Self(speed)
    }
}

/// can query `&Aim` alone. Defaults to `0.0`. `#[serde(transparent)]` lets an
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Aim(f32);

impl Aim {
        #[must_use]
    pub const fn new(aim: f32) -> Self {
        Self(aim)
    }
}

/// `#[serde(transparent)]` lets an authored Strength parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Strength(f32);

impl Strength {
            #[must_use]
    pub const fn new(strength: f32) -> Self {
        Self(strength)
    }
}

/// `0.0`. `#[serde(transparent)]` lets an authored Reflexes parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Reflexes(f32);

impl Reflexes {
            #[must_use]
    pub const fn new(reflexes: f32) -> Self {
        Self(reflexes)
    }
}

/// alone. Defaults to `0.0`. `#[serde(transparent)]` lets an authored Cool parse as
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cool(f32);

impl Cool {
            #[must_use]
    pub const fn new(cool: f32) -> Self {
        Self(cool)
    }
}

/// `#[serde(transparent)]` lets an authored Grit parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grit(f32);

impl Grit {
            #[must_use]
    pub const fn new(grit: f32) -> Self {
        Self(grit)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GangerAttributes {
        pub speed:     Speed,
        pub aim:       Aim,
        pub strength:  Strength,
        pub toughness: Toughness,
        pub reflexes:  Reflexes,
        pub cool:      Cool,
        pub grit:      Grit,
            pub luck:      Luck,
}

