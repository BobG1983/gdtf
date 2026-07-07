//! Where a sprite's PIXELS come from — [`SpriteSource`] and its
//! [`SpriteImagePath`] / [`SpriteRect`] / [`SpritePx`] building blocks.

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// A pixel coordinate or extent in SOURCE-IMAGE space — the unit every
/// sprite-def pixel field shares ([`SpriteRect`]'s sheet-space position/size,
/// [`SpriteAnchor`](super::SpriteAnchor)'s sprite-local offset).
///
/// A named newtype over `u32` (no-bare-types rule 1: a source-pixel quantity
/// is a domain value), [`Deref`]ing to it so a consumer reads the raw pixel
/// count straight through. `#[serde(transparent)]` so an authored field parses
/// as a bare integer (`x: 96`), not a one-field struct.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpritePx(u32);

impl SpritePx {
    /// Wrap a source-image pixel coordinate/extent.
    #[must_use]
    pub const fn new(px: u32) -> Self {
        Self(px)
    }
}

/// An image-asset path a sprite's pixels are read from, relative to the asset
/// source root (e.g. `sprites/alt_tileset_terrain.png`) — either a standalone
/// [`File`](SpriteSource::File) image or the `sheet` a
/// [`Sheet`](SpriteSource::Sheet) source cuts its [`SpriteRect`] out of.
///
/// A named newtype over [`String`] (no-bare-types rule 1: an asset path is a
/// domain value). Private inner + derived [`Deref`]; `#[serde(transparent)]`
/// parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteImagePath(String);

impl SpriteImagePath {
    /// Wrap an asset-root-relative image path.
    #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

/// A rectangle of pixels on a sprite SHEET — the region a
/// [`Sheet`](SpriteSource::Sheet) source draws, positioned from the sheet's
/// TOP-LEFT origin.
///
/// Authored as `rect: (x: 96, y: 0, w: 16, h: 16)`; every field is a
/// [`SpritePx`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteRect {
    /// Left edge of the region, in sheet pixels from the sheet's left.
    pub x: SpritePx,
    /// Top edge of the region, in sheet pixels from the sheet's top.
    pub y: SpritePx,
    /// Width of the region, in pixels.
    pub w: SpritePx,
    /// Height of the region, in pixels.
    pub h: SpritePx,
}

/// Where a sprite's pixels come from — the ruled `source:` field: a standalone
/// image FILE, or a `{sheet, rect}` region cut out of a shared sprite sheet.
///
/// RON forms: `source: File("sprites/lone_crate.png")` or
/// `source: Sheet(sheet: "sprites/alt_tileset_terrain.png", rect: (x: 96, y: 0, w: 16, h: 16))`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum SpriteSource {
    /// The whole standalone image at this path is the sprite.
    File(SpriteImagePath),
    /// A [`SpriteRect`] region of the sheet image at `sheet` is the sprite.
    Sheet {
        /// The sheet image the region is cut from.
        sheet: SpriteImagePath,
        /// The pixel region of `sheet` this sprite draws.
        rect:  SpriteRect,
    },
}
