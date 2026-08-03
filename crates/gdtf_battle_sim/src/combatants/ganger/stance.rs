use bevy::prelude::{Component, Deref};
use serde::Deserialize;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum StanceKind {
        #[default]
    Standing,
        Crouching,
        Prone,
}

/// a tunable). `#[serde(transparent)]` lets an authored stance parse as the bare
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Stance(StanceKind);

impl Stance {
                    #[must_use]
    pub const fn new(posture: StanceKind) -> Self {
        Self(posture)
    }
}

/// `#[serde(transparent)]` lets an authored aim-mode parse as a bare boolean.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Aiming(bool);

impl Aiming {
                    #[must_use]
    pub const fn new(aiming: bool) -> Self {
        Self(aiming)
    }
}

/// `#[serde(transparent)]` lets an authored gang index parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Faction(u8);

impl Faction {
                    #[must_use]
    pub const fn new(gang: u8) -> Self {
        Self(gang)
    }
}
