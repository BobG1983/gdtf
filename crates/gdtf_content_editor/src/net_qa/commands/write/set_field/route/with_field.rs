//! The routes for the nine tabs that own a field arm of their own.

use bevy::prelude::ResMut;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    injuries::InjuryRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    field_form::FieldDraft,
    gang_form::GangDraft,
    injury_form::{InjuryDraft, WeightingDraft},
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::set_field::{
            armor, attachment,
            command::{Written, no_field_of_its_own, present},
            field, gang, injury, melee_weapon, sprite, terrain, weapon, weighting,
        },
        wire::EditorFieldNet,
    },
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    weapon_form::WeaponDraft,
};

/// Route a field write on the Terrain tab, which owns the Terrain form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn terrain(
    draft: Option<&mut ResMut<'_, TerrainDraft>>,
    registries: &terrain::TerrainWriteRegistries<'_>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Terrain(arm) => Ok(EditorFieldNet::Terrain(terrain::write(
            present(draft)?,
            registries,
            arm,
        )?)),
        EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Gang tab, whose members read three content registries.
pub(in crate::net_qa::commands::write::set_field) fn gang(
    draft: Option<&mut ResMut<'_, GangDraft>>,
    weapons: Option<&WeaponRegistry>,
    melee: Option<&MeleeWeaponRegistry>,
    armor: Option<&ArmorRegistry>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Gang(arm) => Ok(EditorFieldNet::Gang(gang::write(
            present(draft)?,
            weapons,
            melee,
            armor,
            arm,
        )?)),
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Armor tab, which owns the Armor form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn armor(
    draft: Option<&mut ResMut<'_, ArmorDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Armor(arm) => {
            Ok(EditorFieldNet::Armor(armor::write(present(draft)?, arm)?))
        }
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Injury tab, which owns its def arms and the weighting sub-tab's.
pub(in crate::net_qa::commands::write::set_field) fn injury(
    injury_draft: Option<&mut ResMut<'_, InjuryDraft>>,
    weighting_draft: Option<&mut ResMut<'_, WeightingDraft>>,
    injuries: Option<&InjuryRegistry>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Injury(arm) => Ok(EditorFieldNet::Injury(injury::write(
            present(injury_draft)?,
            arm,
        )?)),
        EditorFieldNet::Weighting(arm) => Ok(EditorFieldNet::Weighting(weighting::write(
            present(weighting_draft)?,
            injuries,
            arm,
        )?)),
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(injury_draft),
    }
}

/// Route a field write on the Sprite tab, which owns the Sprite form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn sprite(
    draft: Option<&mut ResMut<'_, SpriteDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Sprite(arm) => {
            Ok(EditorFieldNet::Sprite(sprite::write(present(draft)?, arm)?))
        }
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Attachment tab, which owns the Attachment form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn attachment(
    draft: Option<&mut ResMut<'_, AttachmentDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Attachment(arm) => Ok(EditorFieldNet::Attachment(attachment::write(
            present(draft)?,
            arm,
        )?)),
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Weapon tab, which owns the ranged weapon form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn weapon(
    draft: Option<&mut ResMut<'_, WeaponDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Weapon(arm) => {
            Ok(EditorFieldNet::Weapon(weapon::write(present(draft)?, arm)?))
        }
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Melee Weapon tab, which owns the melee form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn melee_weapon(
    draft: Option<&mut ResMut<'_, MeleeWeaponDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::MeleeWeapon(arm) => Ok(EditorFieldNet::MeleeWeapon(melee_weapon::write(
            present(draft)?,
            arm,
        ))),
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Field tab, which owns the field def form's own arms.
pub(in crate::net_qa::commands::write::set_field) fn field(
    draft: Option<&mut ResMut<'_, FieldDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Field(arm) => {
            Ok(EditorFieldNet::Field(field::write(present(draft)?, arm)?))
        }
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Theme(_) => no_field_of_its_own(draft),
    }
}
