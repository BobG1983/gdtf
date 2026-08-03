use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CellXNet(i32);

impl CellXNet {
        #[must_use]
    pub const fn new(x: i32) -> Self {
        Self(x)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CellYNet(i32);

impl CellYNet {
        #[must_use]
    pub const fn new(y: i32) -> Self {
        Self(y)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LevelNet(u8);

impl LevelNet {
        #[must_use]
    pub const fn new(storey: u8) -> Self {
        Self(storey)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellNet {
        pub x: CellXNet,
        pub y: CellYNet,
}

impl CellNet {
        #[must_use]
    pub const fn new(x: CellXNet, y: CellYNet) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellLevelNet {
        pub cell:  CellNet,
        pub level: LevelNet,
}

impl CellLevelNet {
        #[must_use]
    pub const fn new(cell: CellNet, level: LevelNet) -> Self {
        Self { cell, level }
    }
}
