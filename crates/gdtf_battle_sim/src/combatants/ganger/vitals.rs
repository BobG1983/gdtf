//! Runtime vitals and derived combat stats as components.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// Display name.
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerName(String);

impl GangerName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// Current hit points.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Hp(u16);

impl Hp {
    /// Wrap HP.
    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

/// Maximum hit points.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct HpMax(u16);

impl HpMax {
    /// Wrap max HP.
    #[must_use]
    pub const fn new(hp_max: u16) -> Self {
        Self(hp_max)
    }
}

/// Current wounds remaining.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Wounds(u8);

impl Wounds {
    /// Wrap wounds.
    #[must_use]
    pub const fn new(wounds: u8) -> Self {
        Self(wounds)
    }
}

/// Maximum wounds.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct WoundsMax(u8);

impl WoundsMax {
    /// Wrap max wounds.
    #[must_use]
    pub const fn new(wounds_max: u8) -> Self {
        Self(wounds_max)
    }
}

/// Current time units.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Tu(u8);

impl Tu {
    /// Wrap TU.
    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// Maximum time units.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct TuMax(u8);

impl TuMax {
    /// Wrap max TU.
    #[must_use]
    pub const fn new(tu_max: u8) -> Self {
        Self(tu_max)
    }
}

/// Derived shooting skill.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(transparent)]
pub struct Shooting(f32);

impl Shooting {
    /// Wrap shooting.
    #[must_use]
    pub const fn new(shooting: f32) -> Self {
        Self(shooting)
    }
}

/// Toughness (also an authored attribute).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Toughness(f32);

impl Toughness {
    /// Wrap toughness.
    #[must_use]
    pub const fn new(toughness: f32) -> Self {
        Self(toughness)
    }
}

/// Luck (also an authored attribute).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Luck(f32);

impl Luck {
    /// Wrap luck.
    #[must_use]
    pub const fn new(luck: f32) -> Self {
        Self(luck)
    }
}

/// Derived melee fight skill.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Fight(f32);

impl Fight {
    /// Wrap fight.
    #[must_use]
    pub const fn new(fight: f32) -> Self {
        Self(fight)
    }
}

/// Derived reaction skill.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Reactions(f32);

impl Reactions {
    /// Wrap reactions.
    #[must_use]
    pub const fn new(reactions: f32) -> Self {
        Self(reactions)
    }
}

/// Derived morale.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Morale(f32);

impl Morale {
    /// Wrap morale.
    #[must_use]
    pub const fn new(morale: f32) -> Self {
        Self(morale)
    }
}

/// Bottle / break threshold.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bottle(u8);

impl Bottle {
    /// Wrap bottle.
    #[must_use]
    pub const fn new(bottle: u8) -> Self {
        Self(bottle)
    }
}
