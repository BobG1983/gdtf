//! The routes for the three tabs that draw no list of their own.

use bevy::prelude::ResMut;

use crate::{
    armor_form::ArmorDraft,
    net_qa::{
        commands::write::{
            form_fault::FormWriteFault,
            list_op::command::{ListOpRefusal, Routed, no_list_of_its_own, present},
        },
        wire::EditorListNet,
    },
    theme_form::ThemeDraft,
};

const ARMOR_HAS_NO_LIST: &str = "the Armor form draws six fixed pieces and no list at all, so no list name is right while \
     its tab is open";

/// Route a list op on the Theme tab, which draws no list of its own.
pub(in crate::net_qa::commands::write::list_op) fn theme(
    draft: Option<&mut ResMut<'_, ThemeDraft>>,
    list: EditorListNet,
) -> Routed {
    match list {
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::TerrainOnDeathEffects
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
        | EditorListNet::WeaponOnDeathEffects
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => no_list_of_its_own(draft),
    }
}

/// Route a list op on the Armor tab, whose six fixed pieces draw no list at all.
pub(in crate::net_qa::commands::write::list_op) fn armor(
    draft: Option<&mut ResMut<'_, ArmorDraft>>,
    list: EditorListNet,
) -> Routed {
    match list {
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::TerrainOnDeathEffects
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
        | EditorListNet::WeaponOnDeathEffects
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => {
            present(draft)?;
            Err(ListOpRefusal::Fault(FormWriteFault::bad(
                ARMOR_HAS_NO_LIST.to_owned(),
            )))
        }
    }
}

/// Route a list op on the Prefab tab, the map canvas, which holds no draft and no list.
pub(in crate::net_qa::commands::write::list_op) const fn prefab(list: EditorListNet) -> Routed {
    match list {
        EditorListNet::EntrySides
        | EditorListNet::TerrainTags
        | EditorListNet::TerrainOnDeathEffects
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
        | EditorListNet::WeaponOnDeathEffects
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => {
            Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm))
        }
    }
}
