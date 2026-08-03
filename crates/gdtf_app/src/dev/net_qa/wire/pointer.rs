//! Pointer position and mouse buttons on the wire.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Pointer X in screen pixels.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PointerXNet(i16);

impl PointerXNet {
    /// Build from a raw X value.
    #[must_use]
    pub const fn new(x: i16) -> Self {
        Self(x)
    }
}

/// Pointer Y in screen pixels.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PointerYNet(i16);

impl PointerYNet {
    /// Build from a raw Y value.
    #[must_use]
    pub const fn new(y: i16) -> Self {
        Self(y)
    }
}

/// Pointer position on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointerPosNet {
    /// X coordinate.
    pub x: PointerXNet,
    /// Y coordinate.
    pub y: PointerYNet,
}

impl PointerPosNet {
    /// Build from X and Y.
    #[must_use]
    pub const fn new(x: PointerXNet, y: PointerYNet) -> Self {
        Self { x, y }
    }
}

/// Mouse button on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MouseButtonNet {
    /// Left button.
    Left,
    /// Right button.
    Right,
}
