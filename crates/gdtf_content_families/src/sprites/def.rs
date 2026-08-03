//! Sprite definition types: anchor, facing, animation, and source layout.

use std::collections::BTreeMap;

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::source::{SpritePx, SpriteSource};

/// Pixel anchor of a sprite relative to its cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteAnchor {
    /// Horizontal offset in pixels.
    pub x: SpritePx,
    /// Vertical offset in pixels.
    pub y: SpritePx,
}

/// Cardinal facing for multi-facing sprites.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, TypePath,
)]
pub enum SpriteFacing {
    /// Facing north.
    North,
    /// Facing east.
    East,
    /// Facing south.
    South,
    /// Facing west.
    West,
}

/// Per-facing source map.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFacings(BTreeMap<SpriteFacing, SpriteSource>);

impl SpriteFacings {
    /// Build from facing/source pairs.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (SpriteFacing, SpriteSource)>) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// Animation frame rate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFps(f32);

impl SpriteFps {
    /// Wrap an fps value.
    #[must_use]
    pub const fn new(fps: f32) -> Self {
        Self(fps)
    }
}

/// Multi-frame animation definition.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteAnimation {
    /// Frames per second.
    pub fps: SpriteFps,
    /// Ordered frame sources.
    pub frames: Vec<SpriteSource>,
}

/// Authored sprite definition.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteDef {
    /// Default image source.
    pub source: SpriteSource,
    /// Cell anchor.
    pub anchor: SpriteAnchor,
    /// Optional per-facing sources.
    #[serde(default)]
    pub facings: Option<SpriteFacings>,
    /// Optional animation.
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
}
