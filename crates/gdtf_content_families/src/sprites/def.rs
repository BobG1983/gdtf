use std::collections::BTreeMap;

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::source::{SpritePx, SpriteSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteAnchor {
        pub x: SpritePx,
        pub y: SpritePx,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, TypePath,
)]
pub enum SpriteFacing {
        North,
        East,
        South,
        West,
}

/// read through [`Deref`]; `#[serde(transparent)]` so the authored RON is a
#[derive(Deref, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFacings(BTreeMap<SpriteFacing, SpriteSource>);

impl SpriteFacings {
        #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (SpriteFacing, SpriteSource)>) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// domain value), [`Deref`]ing to it; `#[serde(transparent)]` parses a bare
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteFps(f32);

impl SpriteFps {
        #[must_use]
    pub const fn new(fps: f32) -> Self {
        Self(fps)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteAnimation {
        pub fps:    SpriteFps,
        pub frames: Vec<SpriteSource>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct SpriteDef {
        pub source:    SpriteSource,
        pub anchor:    SpriteAnchor,
            #[serde(default)]
    pub facings:   Option<SpriteFacings>,
        #[serde(default)]
    pub animation: Option<SpriteAnimation>,
}
