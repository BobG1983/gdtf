//! The sprite DEFINITION payload — [`SpriteDef`] and its anchor / facings /
//! animation fields (the GTW-600 ruled schema, verbatim: no field drift).

use std::collections::BTreeMap;

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::source::{SpritePx, SpriteSource};

/// A sprite's GROUND-CONTACT / PIVOT point — the authored `anchor: (x, y)`,
/// in sprite-LOCAL pixels measured from the sprite's top-left corner.
///
/// The seeded terrain defs document the CURRENT implicit anchor: the presenter
/// draws each 16×16 tile as a unit quad centered on its cell's world position
/// (`Rectangle::from_size(Vec2::ONE)` at `cell_to_world` — the sprite CENTER
/// sits on the cell center), so the seeds author `(x: 8, y: 8)`. GTW-665 owns
/// actually consuming the anchor in the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteAnchor {
    /// Pixels right of the sprite's LEFT edge.
    pub x: SpritePx,
    /// Pixels below the sprite's TOP edge.
    pub y: SpritePx,
}

/// One of the four cardinal facings a 4-facing tile can author a distinct
/// sprite for — the CLOSED facing vocabulary of the ruled `facings` map.
///
/// `PartialOrd`/`Ord` so the [`SpriteFacings`] map is a deterministic
/// [`BTreeMap`] (stable iteration/serialization order: N, E, S, W).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, TypePath,
)]
pub enum SpriteFacing {
    /// Facing up the map (away from the viewer's bottom edge).
    North,
    /// Facing right.
    East,
    /// Facing down the map (toward the viewer).
    South,
    /// Facing left.
    West,
}

/// The OPTIONAL per-facing source overrides of a 4-facing tile — the ruled
/// `facings` map: each authored [`SpriteFacing`] draws its own
/// [`SpriteSource`] instead of the def's base `source`.
///
/// A named newtype over the facing→source [`BTreeMap`] (no-bare-types rule 1),
/// read through [`Deref`]; `#[serde(transparent)]` so the authored RON is a
/// bare map (`facings: Some({ North: Sheet(…), East: Sheet(…) })`). A facing
/// absent from the map falls back to the def's base `source` (the map need
/// not be total — that is why the entries are overrides, not a replacement).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFacings(BTreeMap<SpriteFacing, SpriteSource>);

impl SpriteFacings {
    /// Build a facings map from `(facing, source)` entries.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (SpriteFacing, SpriteSource)>) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// An animation's playback rate, in frames per second — the ruled
/// `animation.fps` field.
///
/// A named newtype over `f32` (no-bare-types rule 1: a playback rate is a
/// domain value), [`Deref`]ing to it; `#[serde(transparent)]` parses a bare
/// number (`fps: 4.0`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFps(f32);

impl SpriteFps {
    /// Wrap a frames-per-second playback rate.
    #[must_use]
    pub const fn new(fps: f32) -> Self {
        Self(fps)
    }
}

/// The OPTIONAL animation of a sprite def — the ruled `animation {fps,
/// frames}`: an ordered frame sequence played at [`SpriteFps`].
///
/// Each frame is its own [`SpriteSource`], so a sheet-cut sequence and a
/// file-per-frame sequence both author the same shape. NOT `Eq`: `fps` is an
/// `f32`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteAnimation {
    /// Playback rate, frames per second.
    pub fps:    SpriteFps,
    /// The ordered frame sources, played first-to-last then looped.
    pub frames: Vec<SpriteSource>,
}

/// One sprite DEFINITION — the payload of a `content/sprites/
/// <name>.spritedef.ron` member (the GTW-600 ruled schema).
///
/// The sprite's NAME is the member's FILE STEM (stem-keyed — see
/// [`SpriteDefsFamily`](super::SpriteDefsFamily)), never a payload field; a
/// terrain def's `graphic_name` references it by that name (the foreign key
/// the [`validate`](crate::validate) edge checks). NOT `Eq`: the optional
/// [`SpriteAnimation`] carries an `f32` rate.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteDef {
    /// Where the sprite's pixels come from (a file, or a sheet region).
    pub source:    SpriteSource,
    /// The ground-contact / pivot point, in sprite-local pixels.
    pub anchor:    SpriteAnchor,
    /// OPTIONAL per-facing source overrides (a 4-facing tile). Omitted =
    /// `None`: the sprite draws its base `source` for every facing.
    #[serde(default)]
    pub facings:   Option<SpriteFacings>,
    /// OPTIONAL animation. Omitted = `None`: the sprite is a static image.
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
}
