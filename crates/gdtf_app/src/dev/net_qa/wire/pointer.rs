use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PointerXNet(i16);

impl PointerXNet {
        #[must_use]
    pub const fn new(x: i16) -> Self {
        Self(x)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PointerYNet(i16);

impl PointerYNet {
        #[must_use]
    pub const fn new(y: i16) -> Self {
        Self(y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointerPosNet {
        pub x: PointerXNet,
        pub y: PointerYNet,
}

impl PointerPosNet {
        #[must_use]
    pub const fn new(x: PointerXNet, y: PointerYNet) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MouseButtonNet {
        Left,
        Right,
}
