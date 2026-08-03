use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
/// field is a NON-shove weapon (the field is `#[serde(default)]` on the spec, so the
#[derive(
    Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Shove(bool);

impl Shove {
            #[must_use]
    pub const fn new(shove: bool) -> Self {
        Self(shove)
    }
}

/// Private inner + derived [`Deref`]; `#[serde(transparent)]`. A `#[derive(Component)]`
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Silenced(bool);

impl Silenced {
            #[must_use]
    pub const fn new(silenced: bool) -> Self {
        Self(silenced)
    }
}

impl Default for Silenced {
                fn default() -> Self {
        Self(true)
    }
}
