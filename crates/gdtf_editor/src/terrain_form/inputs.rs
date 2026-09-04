//! Text field wrappers for terrain HP and armor values.

use bevy::prelude::Deref;

/// HP value entered in the terrain form.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct HpInput(u32);

impl HpInput {
    /// Build from a raw HP value.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

impl core::fmt::Display for HpInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for HpInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

/// Armor value entered in the terrain form.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ArmorInput(i32);

impl ArmorInput {
    /// Build from a raw armor value.
    #[must_use]
    pub const fn new(armor: i32) -> Self {
        Self(armor)
    }
}

impl core::fmt::Display for ArmorInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for ArmorInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>().map(Self)
    }
}
