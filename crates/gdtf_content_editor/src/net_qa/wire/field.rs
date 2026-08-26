//! Which single-value field a write names, with the value on its own variant.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{
    armor::{
        ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
        BodyPartNet,
    },
    attachment::{AttachmentEffectNet, AttachmentSlotNet},
    list::EditorListIndexNet,
    sprite::{SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet},
    terrain_kind::TerrainKindNet,
};

/// The name a form's own name field holds, for whichever form named it.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorDraftNameNet(String);

impl EditorDraftNameNet {
    /// Wrap a name a client sent or a draft holds.
    pub(in crate::net_qa) fn new(name: &str) -> Self {
        Self(name.to_owned())
    }
}

/// One field of the active mode's draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorFieldNet {
    /// The Terrain draft's kind pick.
    Kind(TerrainKindNet),
    /// The Armor draft's name.
    ArmorName(EditorDraftNameNet),
    /// The Sprite draft's name.
    SpriteName(EditorDraftNameNet),
    /// The Attachment draft's name.
    AttachmentName(EditorDraftNameNet),
    /// One armor piece's damage floor.
    ArmorFloor {
        /// Which piece.
        part:  BodyPartNet,
        /// The floor it is set to.
        value: ArmorFloorNet,
    },
    /// One armor piece's protection.
    ArmorProtection {
        /// Which piece.
        part:  BodyPartNet,
        /// The protection it is set to.
        value: ArmorProtectionNet,
    },
    /// One armor piece's hardness.
    ArmorHardness {
        /// Which piece.
        part:  BodyPartNet,
        /// The hardness it is set to.
        value: ArmorHardnessNet,
    },
    /// One armor piece's integrity.
    ArmorIntegrity {
        /// Which piece.
        part:  BodyPartNet,
        /// The integrity it is set to.
        value: ArmorIntegrityNet,
    },
    /// One armor piece's material.
    ArmorType {
        /// Which piece.
        part:  BodyPartNet,
        /// The material it is set to.
        value: ArmorTypeNet,
    },
    /// The Sprite draft's base image source.
    SpriteBaseSource(SpriteSourceNet),
    /// The Sprite draft's anchor, horizontal axis.
    SpriteAnchorX(SpritePxNet),
    /// The Sprite draft's anchor, vertical axis.
    SpriteAnchorY(SpritePxNet),
    /// The Sprite animation's frame rate.
    SpriteFps(SpriteFpsNet),
    /// One facing's source override, set or cleared.
    SpriteFacingOverride {
        /// Which facing.
        facing: SpriteFacingNet,
        /// The source it overrides with, or none to clear it.
        source: Option<SpriteSourceNet>,
    },
    /// One animation frame's source.
    SpriteFrame {
        /// Which frame.
        index:  EditorListIndexNet,
        /// The source it is set to.
        source: SpriteSourceNet,
    },
    /// Whether the Sprite draft is animated.
    SpriteAnimated(SpriteAnimatedNet),
    /// The Attachment draft's display name.
    AttachmentDisplayName(EditorDraftNameNet),
    /// The Attachment draft's mounting slot.
    AttachmentSlot(AttachmentSlotNet),
    /// One authored effect, variant and payload together.
    AttachmentEffect {
        /// Which effect.
        index:  EditorListIndexNet,
        /// The effect it is set to.
        effect: AttachmentEffectNet,
    },
}
