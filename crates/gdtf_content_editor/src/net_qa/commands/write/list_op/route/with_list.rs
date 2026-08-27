//! The routes for the eight tabs that draw a list of their own.

use bevy::prelude::ResMut;
use gdtf_battle_sim::{equipment::attachments::AttachmentRegistry, injuries::InjuryRegistry};

use crate::{
    attachment_form::AttachmentDraft,
    field_form::FieldDraft,
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::list_op::{
            attachment,
            command::{Routed, no_list_of_its_own, present},
            field, gang, injury, melee_weapon, sprite, terrain, weapon, weighting,
        },
        wire::{EditorListNet, EditorListOpNet},
    },
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    weapon_form::WeaponDraft,
};

/// Route a list op on the Terrain tab, which owns the entry sides and the tag tick boxes.
pub(in crate::net_qa::commands::write::list_op) fn terrain(
    draft: Option<&mut ResMut<'_, TerrainDraft>>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::EntrySides | EditorListNet::TerrainTags => {
            let draft = present(draft)?;
            terrain::apply(draft, list, op)?;
            Ok(terrain::members(draft, list))
        }
        EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Injury tab, which owns its effects and the weighting buckets.
pub(in crate::net_qa::commands::write::list_op) fn injury(
    injury_draft: Option<&mut ResMut<'_, InjuryDraft>>,
    weighting_draft: Option<&mut ResMut<'_, WeightingDraft>>,
    registry: Option<&InjuryRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::InjuryEffects => {
            let draft = present(injury_draft)?;
            injury::apply(draft, op)?;
            Ok(injury::members(draft))
        }
        EditorListNet::WeightingBucket(bucket) => {
            let draft = present(weighting_draft)?;
            weighting::apply(draft, registry, bucket, op)?;
            Ok(weighting::members(draft, bucket))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes => no_list_of_its_own(injury_draft),
    }
}

/// Route a list op on the Melee Weapon tab, which owns its fight modes, slots and attachments.
pub(in crate::net_qa::commands::write::list_op) fn melee_weapon(
    draft: Option<&mut ResMut<'_, MeleeWeaponDraft>>,
    registry: Option<&AttachmentRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments => {
            let draft = present(draft)?;
            melee_weapon::apply(draft, registry, list, op)?;
            Ok(melee_weapon::members(draft, list))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Weapon tab, which owns its fire modes, slots and attachments.
pub(in crate::net_qa::commands::write::list_op) fn weapon(
    draft: Option<&mut ResMut<'_, WeaponDraft>>,
    registry: Option<&AttachmentRegistry>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments => {
            let draft = present(draft)?;
            weapon::apply(draft, registry, list, op)?;
            Ok(weapon::members(draft, list))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Attachment tab, which owns its authored effects.
pub(in crate::net_qa::commands::write::list_op) fn attachment(
    draft: Option<&mut ResMut<'_, AttachmentDraft>>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::AttachmentEffects => {
            let draft = present(draft)?;
            attachment::apply(draft, op)?;
            Ok(attachment::members(draft))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Gang tab, which owns its roster of members.
pub(in crate::net_qa::commands::write::list_op) fn gang(
    draft: Option<&mut ResMut<'_, GangDraft>>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::GangMembers => {
            let draft = present(draft)?;
            gang::apply(draft, op)?;
            Ok(gang::members(draft))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Sprite tab, which owns its animation frames.
pub(in crate::net_qa::commands::write::list_op) fn sprite(
    draft: Option<&mut ResMut<'_, SpriteDraft>>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::SpriteFrames => {
            let draft = present(draft)?;
            sprite::apply(draft, op)?;
            Ok(sprite::members(draft))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Field tab, which owns its immune-armor tick boxes.
pub(in crate::net_qa::commands::write::list_op) fn field(
    draft: Option<&mut ResMut<'_, FieldDraft>>,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match list {
        EditorListNet::FieldImmuneArmorTypes => {
            let draft = present(draft)?;
            field::apply(draft, op)?;
            Ok(field::members(draft))
        }
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}
