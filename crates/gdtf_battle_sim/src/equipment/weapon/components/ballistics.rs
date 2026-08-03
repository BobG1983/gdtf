use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar. A
/// `#[derive(Component)]` so it lives as a sibling component on the armed entity
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct BaseSpread(f32);

impl BaseSpread {
        #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// §1b). Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Accuracy(f32);

impl Accuracy {
        #[must_use]
    pub const fn new(accuracy: f32) -> Self {
        Self(accuracy)
    }
}

/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Kickback(f32);

impl Kickback {
        #[must_use]
    pub const fn new(kickback: f32) -> Self {
        Self(kickback)
    }
}

/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Stable(bool);

impl Stable {
            #[must_use]
    pub const fn new(stable: bool) -> Self {
        Self(stable)
    }
}
