//! The Gang draft's name and its members' rows, written the way the member panel writes them.

use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::{Aim, Cool, GangMember, GangerName, Grit, Luck, Reflexes, Speed, Strength, Toughness},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

use super::registry;
use crate::{
    gang_form::GangDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            EditorDraftNameNet, EditorFieldNet, EditorKeyNet, EditorListIndexNet, GangAttributeNet,
            GangAttributeValueNet,
        },
    },
};

// The member that index names, or the fault an index past the end of the roster answers.
fn member_at(
    draft: &mut GangDraft,
    index: EditorListIndexNet,
) -> Result<&mut GangMember, FormWriteFault> {
    let held = draft.members().len();
    draft.members_mut().get_mut(*index).ok_or_else(|| {
        FormWriteFault::bad(format!(
            "member {} is past the end of a roster holding {held}",
            *index
        ))
    })
}

// The value the attribute drag would accept, or the fault one outside its range answers.
fn in_range(value: GangAttributeValueNet) -> Result<GangAttributeValueNet, FormWriteFault> {
    if GangDraft::ATTRIBUTE_RANGE.contains(&*value) {
        Ok(value)
    } else {
        Err(FormWriteFault::bad(format!(
            "{} is outside the {}..={} range the attribute drag offers",
            *value,
            GangDraft::ATTRIBUTE_RANGE.start(),
            GangDraft::ATTRIBUTE_RANGE.end(),
        )))
    }
}

// Write one attribute, answering the value the member holds afterwards.
fn store_attribute(
    member: &mut GangMember,
    attribute: GangAttributeNet,
    value: GangAttributeValueNet,
) -> GangAttributeValueNet {
    let stored = match attribute {
        GangAttributeNet::Speed => {
            member.speed = Speed::new(*value);
            *member.speed
        }
        GangAttributeNet::Aim => {
            member.aim = Aim::new(*value);
            *member.aim
        }
        GangAttributeNet::Strength => {
            member.strength = Strength::new(*value);
            *member.strength
        }
        GangAttributeNet::Toughness => {
            member.toughness = Toughness::new(*value);
            *member.toughness
        }
        GangAttributeNet::Reflexes => {
            member.reflexes = Reflexes::new(*value);
            *member.reflexes
        }
        GangAttributeNet::Cool => {
            member.cool = Cool::new(*value);
            *member.cool
        }
        GangAttributeNet::Grit => {
            member.grit = Grit::new(*value);
            *member.grit
        }
        GangAttributeNet::Luck => {
            member.luck = Luck::new(*value);
            *member.luck
        }
    };
    GangAttributeValueNet::new(stored)
}

fn write_member_name(
    draft: &mut GangDraft,
    index: EditorListIndexNet,
    name: &EditorDraftNameNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let member = member_at(draft, index)?;
    member.name = GangerName::new((**name).clone());
    Ok(EditorFieldNet::GangMemberName {
        index,
        name: EditorDraftNameNet::new(member.name.as_str()),
    })
}

fn write_attribute(
    draft: &mut GangDraft,
    index: EditorListIndexNet,
    attribute: GangAttributeNet,
    value: GangAttributeValueNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let wanted = in_range(value)?;
    let member = member_at(draft, index)?;
    Ok(EditorFieldNet::GangMemberAttribute {
        index,
        attribute,
        value: store_attribute(member, attribute, wanted),
    })
}

fn write_weapon(
    draft: &mut GangDraft,
    weapons: Option<&WeaponRegistry>,
    index: EditorListIndexNet,
    key: &EditorKeyNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let wanted = registry::weapon(weapons, key)?;
    let member = member_at(draft, index)?;
    member.weapon = wanted;
    Ok(EditorFieldNet::GangMemberWeapon {
        index,
        key: EditorKeyNet::new(member.weapon.as_str().to_owned()),
    })
}

fn write_armor(
    draft: &mut GangDraft,
    armor: Option<&ArmorRegistry>,
    index: EditorListIndexNet,
    key: &EditorKeyNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let wanted = registry::armor(armor, key)?;
    let member = member_at(draft, index)?;
    member.armor = wanted;
    Ok(EditorFieldNet::GangMemberArmor {
        index,
        key: EditorKeyNet::new(member.armor.as_str().to_owned()),
    })
}

fn write_melee_weapon(
    draft: &mut GangDraft,
    melee: Option<&MeleeWeaponRegistry>,
    index: EditorListIndexNet,
    key: Option<&EditorKeyNet>,
) -> Result<EditorFieldNet, FormWriteFault> {
    let wanted = match key {
        Some(key) => Some(registry::melee_weapon(melee, key)?),
        None => None,
    };
    let member = member_at(draft, index)?;
    member.melee_weapon = wanted;
    Ok(EditorFieldNet::GangMemberMeleeWeapon {
        index,
        key: member
            .melee_weapon
            .as_ref()
            .map(|name| EditorKeyNet::new(name.as_str().to_owned())),
    })
}

/// Write one Gang field, answering the field as the draft stores it.
pub(in crate::net_qa::commands::write::set_field) fn write(
    draft: &mut GangDraft,
    weapons: Option<&WeaponRegistry>,
    melee: Option<&MeleeWeaponRegistry>,
    armor: Option<&ArmorRegistry>,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::GangName(name) => {
            draft.set_name((*name).clone());
            Ok(EditorFieldNet::GangName(EditorDraftNameNet::new(
                draft.name(),
            )))
        }
        EditorFieldNet::GangMemberName { index, name } => write_member_name(draft, index, &name),
        EditorFieldNet::GangMemberAttribute {
            index,
            attribute,
            value,
        } => write_attribute(draft, index, attribute, value),
        EditorFieldNet::GangMemberWeapon { index, key } => {
            write_weapon(draft, weapons, index, &key)
        }
        EditorFieldNet::GangMemberArmor { index, key } => write_armor(draft, armor, index, &key),
        EditorFieldNet::GangMemberMeleeWeapon { index, key } => {
            write_melee_weapon(draft, melee, index, key.as_ref())
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
