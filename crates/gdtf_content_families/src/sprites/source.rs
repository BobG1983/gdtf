//! Sprite image path, pixel size, and sheet rectangle types.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// Pixel count on one axis.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpritePx(u32);

impl SpritePx {
    /// Wrap a pixel count.
    #[must_use]
    pub const fn new(px: u32) -> Self {
        Self(px)
    }
}

/// Path to a sprite image asset.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteImagePath(String);

impl SpriteImagePath {
    /// Wrap a path string.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

/// Rectangle within a sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteRect {
    /// Left edge in pixels.
    pub x: SpritePx,
    /// Top edge in pixels.
    pub y: SpritePx,
    /// Width in pixels.
    pub w: SpritePx,
    /// Height in pixels.
    pub h: SpritePx,
}

/// Where a sprite's pixels come from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum SpriteSource {
    /// Standalone image file.
    File(SpriteImagePath),
    /// Region of a sprite sheet.
    Sheet {
        /// Sheet image path.
        sheet: SpriteImagePath,
        /// Region inside the sheet.
        rect: SpriteRect,
    },
}
