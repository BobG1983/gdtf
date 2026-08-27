//! The Sprite form's own fields, one arm per control its panels draw.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::{
    list::EditorListIndexNet,
    sprite::{SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet},
};

/// One field of the Sprite draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum SpriteFieldNet {
    /// The draft's name.
    Name(EditorDraftNameNet),
    /// The draft's base image source.
    BaseSource(SpriteSourceNet),
    /// The draft's anchor, horizontal axis.
    AnchorX(SpritePxNet),
    /// The draft's anchor, vertical axis.
    AnchorY(SpritePxNet),
    /// The animation's frame rate.
    Fps(SpriteFpsNet),
    /// One facing's source override, set or cleared.
    FacingOverride {
        /// Which facing.
        facing: SpriteFacingNet,
        /// The source it overrides with, or none to clear it.
        source: Option<SpriteSourceNet>,
    },
    /// One animation frame's source.
    Frame {
        /// Which frame.
        index:  EditorListIndexNet,
        /// The source it is set to.
        source: SpriteSourceNet,
    },
    /// Whether the draft is animated.
    Animated(SpriteAnimatedNet),
}
