use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// count straight through. `#[serde(transparent)]` so an authored field parses
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpritePx(u32);

impl SpritePx {
        #[must_use]
    pub const fn new(px: u32) -> Self {
        Self(px)
    }
}

/// domain value). Private inner + derived [`Deref`]; `#[serde(transparent)]`
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct SpriteImagePath(String);

impl SpriteImagePath {
        #[must_use]
    pub const fn new(path: String) -> Self {
        Self(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct SpriteRect {
        pub x: SpritePx,
        pub y: SpritePx,
        pub w: SpritePx,
        pub h: SpritePx,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub enum SpriteSource {
        File(SpriteImagePath),
        Sheet {
                sheet: SpriteImagePath,
                rect:  SpriteRect,
    },
}
