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
            EditorDraftNameNet, EditorKeyNet, EditorListIndexNet, GangAttributeNet,
            GangAttributeValueNet, GangFieldNet,
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
) -> Result<GangFieldNet, FormWriteFault> {
    let member = member_at(draft, index)?;
    member.name = GangerName::new((**name).clone());
    Ok(GangFieldNet::MemberName {
        index,
        name: EditorDraftNameNet::new(member.name.as_str()),
    })
}

fn write_attribute(
    draft: &mut GangDraft,
    index: EditorListIndexNet,
    attribute: GangAttributeNet,
    value: GangAttributeValueNet,
) -> Result<GangFieldNet, FormWriteFault> {
    let wanted = in_range(value)?;
    let member = member_at(draft, index)?;
    Ok(GangFieldNet::MemberAttribute {
        index,
        attribute,
        value: store_attribute(member, attribute, wanted),
    })
}

fn write_weapon(
    draft: &mut GangDraft,
    weapons: Option<&WeaponRegistry>,
    index: EditorListIndexNet,
    key: Option<&EditorKeyNet>,
) -> Result<GangFieldNet, FormWriteFault> {
    let wanted = match key {
        Some(key) => Some(registry::weapon(weapons, key)?),
        None => None,
    };
    let member = member_at(draft, index)?;
    member.weapon = wanted;
    Ok(GangFieldNet::MemberWeapon {
        index,
        key: member
            .weapon
            .as_ref()
            .map(|name| EditorKeyNet::new(name.as_str().to_owned())),
    })
}

fn write_armor(
    draft: &mut GangDraft,
    armor: Option<&ArmorRegistry>,
    index: EditorListIndexNet,
    key: Option<&EditorKeyNet>,
) -> Result<GangFieldNet, FormWriteFault> {
    let wanted = match key {
        Some(key) => Some(registry::armor(armor, key)?),
        None => None,
    };
    let member = member_at(draft, index)?;
    member.armor = wanted;
    Ok(GangFieldNet::MemberArmor {
        index,
        key: member
            .armor
            .as_ref()
            .map(|name| EditorKeyNet::new(name.as_str().to_owned())),
    })
}

fn write_melee_weapon(
    draft: &mut GangDraft,
    melee: Option<&MeleeWeaponRegistry>,
    index: EditorListIndexNet,
    key: Option<&EditorKeyNet>,
) -> Result<GangFieldNet, FormWriteFault> {
    let wanted = match key {
        Some(key) => Some(registry::melee_weapon(melee, key)?),
        None => None,
    };
    let member = member_at(draft, index)?;
    member.melee_weapon = wanted;
    Ok(GangFieldNet::MemberMeleeWeapon {
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
    field: GangFieldNet,
) -> Result<GangFieldNet, FormWriteFault> {
    match field {
        GangFieldNet::Name(name) => {
            draft.set_name((*name).clone());
            Ok(GangFieldNet::Name(EditorDraftNameNet::new(draft.name())))
        }
        GangFieldNet::MemberName { index, name } => write_member_name(draft, index, &name),
        GangFieldNet::MemberAttribute {
            index,
            attribute,
            value,
        } => write_attribute(draft, index, attribute, value),
        GangFieldNet::MemberWeapon { index, key } => {
            write_weapon(draft, weapons, index, key.as_ref())
        }
        GangFieldNet::MemberArmor { index, key } => write_armor(draft, armor, index, key.as_ref()),
        GangFieldNet::MemberMeleeWeapon { index, key } => {
            write_melee_weapon(draft, melee, index, key.as_ref())
        }
    }
}
