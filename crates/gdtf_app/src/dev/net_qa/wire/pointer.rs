//! The pointer-position wire types — [`PointerXNet`] / [`PointerYNet`] and the composed
//! [`PointerPosNet`] window coordinate (GTW-783).
//!
//! The wire mirror of a window-space cursor position in LOGICAL pixels — the coordinate a
//! [`Hover`](super::act::NetIntent::Hover) drives the pointer to. Kept as INDEPENDENT
//! serde types (never a leak of a `bevy_math` `Vec2`, so this crate stays bevy-free), and
//! each axis is an `i16`: deliberately INTEGER (whole-pixel precision is enough to place a
//! hover over a UI node, and an integer keeps [`NetIntent`](super::act::NetIntent)
//! `Eq`/`Hash` — an `f32` would drop both across the whole act vocabulary), and deliberately
//! `i16` so the game side widens it to `f32` LOSSLESSLY (`f32::from`, no truncating cast) —
//! its ±32767 range comfortably covers any window's pixel extent. Each axis is its own
//! newtype (no-bare-types rule 3: an x is not a y).

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A pointer's window-space **x** position, in logical pixels — the wire mirror of a
/// cursor position's x axis.
///
/// A private-inner newtype (no-bare-types), serde-transparent so it rides the wire as its
/// bare `i16`. Signed: a hover can be expressed left of the window origin even though a
/// position outside the window resolves to no hover.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct PointerXNet(i16);

impl PointerXNet {
    /// Build a pointer x-position from its logical-pixel value.
    #[must_use]
    pub(crate) const fn new(x: i16) -> Self {
        Self(x)
    }
}

/// A pointer's window-space **y** position, in logical pixels — the wire mirror of a
/// cursor position's y axis.
///
/// A private-inner newtype (no-bare-types), serde-transparent. Distinct from
/// [`PointerXNet`] by rule 3 even though both wrap `i16`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct PointerYNet(i16);

impl PointerYNet {
    /// Build a pointer y-position from its logical-pixel value.
    #[must_use]
    pub(crate) const fn new(y: i16) -> Self {
        Self(y)
    }
}

/// A window-space pointer position — the `(x, y)` logical-pixel pair a
/// [`Hover`](super::act::NetIntent::Hover) moves the cursor to.
///
/// A named-field struct (not a bare tuple) so each axis keeps its meaning; its two fields
/// are the typed [`PointerXNet`] / [`PointerYNet`] scalars. Serde default shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) struct PointerPosNet {
    /// The pointer's window-space x position.
    pub x: PointerXNet,
    /// The pointer's window-space y position.
    pub y: PointerYNet,
}

impl PointerPosNet {
    /// Build a window pointer position from its `x`/`y` logical-pixel coordinates.
    #[must_use]
    pub(crate) const fn new(x: PointerXNet, y: PointerYNet) -> Self {
        Self { x, y }
    }
}
