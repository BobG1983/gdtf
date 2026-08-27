//! Which list a write names, what it does to it, and how its members read back.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{
    armor::ArmorTypeNet,
    attachment::AttachmentEffectNet,
    facing::TerrainFacingNet,
    field::EditorDraftNameNet,
    fire_mode::FireModeSpecNet,
    injury::InjuryEffectNet,
    melee_weapon::{AttachmentKeyNet, FightModeSpecNet, WeaponSlotNet},
    sprite::SpriteSourceNet,
    terrain::TerrainTagNet,
    weighting::{WeightingBucketNet, WeightingRowNet},
};

/// A position in one of the lists a form draws.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorListIndexNet(usize);

impl EditorListIndexNet {
    /// Wrap a position a client sent.
    pub(in crate::net_qa) const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// One list-valued field of the active mode's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorListNet {
    /// The Terrain draft's emplacement entry sides.
    EntrySides,
    /// The Terrain draft's tag tick boxes.
    TerrainTags,
    /// The Attachment draft's authored effects.
    AttachmentEffects,
    /// The Sprite draft's animation frames.
    SpriteFrames,
    /// The Injury draft's authored effects.
    InjuryEffects,
    /// The Melee Weapon draft's fight modes.
    MeleeWeaponFightModes,
    /// The Melee Weapon draft's slot declarations.
    MeleeWeaponSlots,
    /// The Melee Weapon draft's fitted attachment keys.
    MeleeWeaponAttachments,
    /// The Gang draft's roster of members.
    GangMembers,
    /// The Weapon draft's fire modes.
    WeaponFireModes,
    /// The Weapon draft's slot declarations.
    WeaponSlots,
    /// The Weapon draft's fitted attachment keys.
    WeaponAttachments,
    /// The Field draft's immune-armor tick boxes.
    FieldImmuneArmorTypes,
    /// One of the Injury weighting table's three bucket row groups.
    WeightingBucket(WeightingBucketNet),
}

/// What a write does to the named list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorListOpNet {
    /// Add the member if absent, remove it if present.
    Toggle(EditorListMemberNet),
    /// Append the member the form's own Add button appends.
    Add,
    /// Remove the member at one position.
    Remove(EditorListIndexNet),
    /// Rewrite the member at one position.
    SetAt(EditorListIndexNet, EditorListMemberNet),
    /// Move the member at one position a slot toward the start.
    MoveUp(EditorListIndexNet),
    /// Move the member at one position a slot toward the end.
    MoveDown(EditorListIndexNet),
}

/// One member of the list a reply reads back, and the payload a toggle or edit carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorListMemberNet {
    /// One side of the Terrain draft's entry-sides list.
    EntrySide(TerrainFacingNet),
    /// One tag of the Terrain draft's tick-box row.
    TerrainTag(TerrainTagNet),
    /// One effect of the Attachment draft's effects list.
    AttachmentEffect(AttachmentEffectNet),
    /// One frame source of the Sprite draft's animation.
    SpriteFrame(SpriteSourceNet),
    /// One effect of the Injury draft's effects list.
    InjuryEffect(InjuryEffectNet),
    /// One mode of the Melee Weapon draft's fight-mode list.
    FightMode(FightModeSpecNet),
    /// One declaration of the Melee Weapon draft's slot list.
    Slot(WeaponSlotNet),
    /// One key of the Melee Weapon draft's fitted attachment list.
    Attachment(AttachmentKeyNet),
    /// One member of the Gang draft's roster, named the way its header names it.
    GangMember(EditorDraftNameNet),
    /// One mode of the Weapon draft's fire-mode list, every field its row draws.
    FireMode(FireModeSpecNet),
    /// One armor type of the Field draft's immunity tick-box row.
    ImmuneArmorType(ArmorTypeNet),
    /// One row of an Injury weighting bucket, its injury key and its weight.
    WeightingRow(WeightingRowNet),
}
