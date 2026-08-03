use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// `#[serde(transparent)]` lets an authored situation `.ron`'s `name` parse as a
/// serde-transparent shape). A `#[derive(Component)]` so the status panel can query
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GangerName(String);

impl GangerName {
                    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// system can query `&mut Hp` alone. Defaults to `0`. `#[serde(transparent)]`
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Hp(u16);

impl Hp {
                    #[must_use]
    pub const fn new(hp: u16) -> Self {
        Self(hp)
    }
}

/// `#[serde(transparent)]` lets an authored HP ceiling parse as a bare integer (the
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct HpMax(u16);

impl HpMax {
                        #[must_use]
    pub const fn new(hp_max: u16) -> Self {
        Self(hp_max)
    }
}

/// `#[serde(transparent)]` lets an authored Wounds pool parse as a bare integer.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Wounds(u8);

impl Wounds {
                    #[must_use]
    pub const fn new(wounds: u8) -> Self {
        Self(wounds)
    }
}

/// `#[serde(transparent)]` lets an authored Wounds ceiling parse as a bare integer (the
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct WoundsMax(u8);

impl WoundsMax {
                        #[must_use]
    pub const fn new(wounds_max: u8) -> Self {
        Self(wounds_max)
    }
}

/// `&mut Tu` alone. Defaults to `0`. `#[serde(transparent)]` lets an authored TU
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct Tu(u8);

impl Tu {
                    #[must_use]
    pub const fn new(tu: u8) -> Self {
        Self(tu)
    }
}

/// `#[serde(transparent)]` lets an authored TU ceiling parse as a bare integer (the
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct TuMax(u8);

impl TuMax {
                    #[must_use]
    pub const fn new(tu_max: u8) -> Self {
        Self(tu_max)
    }
}

/// `#[serde(transparent)]` lets an authored Shooting stat parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(transparent)]
pub struct Shooting(f32);

impl Shooting {
                            #[must_use]
    pub const fn new(shooting: f32) -> Self {
        Self(shooting)
    }
}

/// can query `&Toughness` alone. Defaults to `0.0`. `#[serde(transparent)]` lets
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Toughness(f32);

impl Toughness {
                        #[must_use]
    pub const fn new(toughness: f32) -> Self {
        Self(toughness)
    }
}

/// `#[serde(transparent)]` lets an authored Luck stat parse as a bare scalar.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Luck(f32);

impl Luck {
                        #[must_use]
    pub const fn new(luck: f32) -> Self {
        Self(luck)
    }
}

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Fight(f32);

impl Fight {
                #[must_use]
    pub const fn new(fight: f32) -> Self {
        Self(fight)
    }
}

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Reactions(f32);

impl Reactions {
                #[must_use]
    pub const fn new(reactions: f32) -> Self {
        Self(reactions)
    }
}

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Morale(f32);

impl Morale {
                #[must_use]
    pub const fn new(morale: f32) -> Self {
        Self(morale)
    }
}

#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bottle(u8);

impl Bottle {
            #[must_use]
    pub const fn new(bottle: u8) -> Self {
        Self(bottle)
    }
}
