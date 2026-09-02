//! The Sprite form's facings, image sources and pixel values on the wire.

use bevy::prelude::Deref;
use gdtf_content_families::sprites::{
    SpriteFacing, SpriteFps, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};
use serde::{Deserialize, Serialize};

/// A cardinal facing a sprite may override its source for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum SpriteFacingNet {
    /// Facing north.
    North,
    /// Facing east.
    East,
    /// Facing south.
    South,
    /// Facing west.
    West,
}

impl SpriteFacingNet {
    /// Every facing a sprite def keys an override by, for a case that walks them all.
    #[cfg(test)]
    pub(in crate::net_qa) const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    /// Mirror the family's own facing.
    pub(in crate::net_qa) const fn from_facing(facing: SpriteFacing) -> Self {
        match facing {
            SpriteFacing::North => Self::North,
            SpriteFacing::East => Self::East,
            SpriteFacing::South => Self::South,
            SpriteFacing::West => Self::West,
        }
    }

    /// Read a client's facing back as the family's own.
    pub(in crate::net_qa) const fn to_facing(self) -> SpriteFacing {
        match self {
            Self::North => SpriteFacing::North,
            Self::East => SpriteFacing::East,
            Self::South => SpriteFacing::South,
            Self::West => SpriteFacing::West,
        }
    }
}

/// A pixel count on one axis.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SpritePxNet(u32);

impl SpritePxNet {
    /// Mirror the family's own pixel count.
    pub(in crate::net_qa) fn from_px(px: SpritePx) -> Self {
        Self(*px)
    }

    /// Read a client's pixel count back as the family's own.
    pub(in crate::net_qa) const fn to_px(self) -> SpritePx {
        SpritePx::new(self.0)
    }
}

/// An animation's frame rate.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SpriteFpsNet(f32);

impl SpriteFpsNet {
    /// Mirror the family's own frame rate.
    pub(in crate::net_qa) fn from_fps(fps: SpriteFps) -> Self {
        Self(*fps)
    }

    /// Read a client's frame rate back as the family's own.
    pub(in crate::net_qa) const fn to_fps(self) -> SpriteFps {
        SpriteFps::new(self.0)
    }
}

/// Whether the sprite draft's animation is switched on.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SpriteAnimatedNet(bool);

impl SpriteAnimatedNet {
    /// Wrap the animated flag.
    pub(in crate::net_qa) const fn new(animated: bool) -> Self {
        Self(animated)
    }
}

/// A path to a sprite image asset.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SpriteImagePathNet(String);

impl SpriteImagePathNet {
    /// Mirror the family's own path.
    pub(in crate::net_qa) fn from_path(path: &SpriteImagePath) -> Self {
        Self((**path).clone())
    }

    /// Read a client's path back as the family's own.
    pub(in crate::net_qa) fn to_path(&self) -> SpriteImagePath {
        SpriteImagePath::new(self.0.clone())
    }
}

/// A sprite registry key, as the name text a client sends.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SpriteKeyNet(String);

impl SpriteKeyNet {
    /// Wrap a sprite registry key.
    pub(in crate::net_qa) const fn new(key: String) -> Self {
        Self(key)
    }
}

/// A rectangle inside a sprite sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) struct SpriteRectNet {
    /// Left edge.
    x: SpritePxNet,
    /// Top edge.
    y: SpritePxNet,
    /// Width.
    w: SpritePxNet,
    /// Height.
    h: SpritePxNet,
}

impl SpriteRectNet {
    /// Mirror the family's own rect.
    pub(in crate::net_qa) fn from_rect(rect: SpriteRect) -> Self {
        Self {
            x: SpritePxNet::from_px(rect.x),
            y: SpritePxNet::from_px(rect.y),
            w: SpritePxNet::from_px(rect.w),
            h: SpritePxNet::from_px(rect.h),
        }
    }

    /// Read a client's rect back as the family's own.
    pub(in crate::net_qa) const fn to_rect(self) -> SpriteRect {
        SpriteRect {
            x: self.x.to_px(),
            y: self.y.to_px(),
            w: self.w.to_px(),
            h: self.h.to_px(),
        }
    }
}

/// Where a sprite's pixels come from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum SpriteSourceNet {
    /// A standalone image file.
    File(SpriteImagePathNet),
    /// A region of a sprite sheet.
    Sheet {
        /// The sheet's own path.
        sheet: SpriteImagePathNet,
        /// The region inside it.
        rect:  SpriteRectNet,
    },
}

impl SpriteSourceNet {
    /// Mirror the family's own source.
    pub(in crate::net_qa) fn from_source(source: &SpriteSource) -> Self {
        match source {
            SpriteSource::File(path) => Self::File(SpriteImagePathNet::from_path(path)),
            SpriteSource::Sheet { sheet, rect } => Self::Sheet {
                sheet: SpriteImagePathNet::from_path(sheet),
                rect:  SpriteRectNet::from_rect(*rect),
            },
        }
    }

    /// Read a client's source back as the family's own.
    pub(in crate::net_qa) fn to_source(&self) -> SpriteSource {
        match self {
            Self::File(path) => SpriteSource::File(path.to_path()),
            Self::Sheet { sheet, rect } => SpriteSource::Sheet {
                sheet: sheet.to_path(),
                rect:  rect.to_rect(),
            },
        }
    }
}
