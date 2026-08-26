//! Look a key up in one family's registry and call that draft's own `load_*` method.

use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry},
    effects::fields::{FieldDefRegistry, FieldKey},
    equipment::attachments::{AttachmentName, AttachmentRegistry},
    ganger::{GangName, GangRegistry},
    injuries::{InjuryName, InjuryRegistry},
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::{
    armor_form::ArmorDraft, attachment_form::AttachmentDraft, field_form::FieldDraft,
    gang_form::GangDraft, injury_form::InjuryDraft, melee_weapon_form::MeleeWeaponDraft,
    net_qa::wire::EditorKeyNet, sprite_form::SpriteDraft, weapon_form::WeaponDraft,
};

/// Whether the key was found and loaded, or the registry holds no such entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::net_qa::commands::write::load) enum KeyLookup {
    /// The entry was loaded into the draft.
    Loaded,
    /// The registry holds no entry under that key; these are the keys it does hold.
    NoSuchKey(Vec<EditorKeyNet>),
}

// Sorted so a client reading a miss sees the same list every time.
fn known_keys(keys: impl Iterator<Item = EditorKeyNet>) -> Vec<EditorKeyNet> {
    let mut known: Vec<EditorKeyNet> = keys.collect();
    known.sort();
    known
}

// The key a client sent, as the string a family's own name newtype is built from.
fn asked_for(key: &EditorKeyNet) -> String {
    (**key).clone()
}

pub(in crate::net_qa::commands::write::load) fn load_gang(
    draft: &mut GangDraft,
    registry: &GangRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = GangName::new(asked_for(key));
    let Some(roster) = registry.roster(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let roster = roster.clone();
    draft.load_gang(&name, &roster);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_armor(
    draft: &mut ArmorDraft,
    registry: &ArmorRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = ArmorName::new(asked_for(key));
    let Some(spec) = registry.spec(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let spec = *spec;
    draft.load_armor(&name, &spec);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_injury(
    draft: &mut InjuryDraft,
    registry: &InjuryRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = InjuryName::new(asked_for(key));
    let Some(def) = registry.def(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .iter()
                .map(|(key, _)| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let def = def.clone();
    draft.load_injury(&name, &def);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_sprite(
    draft: &mut SpriteDraft,
    registry: &SpriteDefRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = SpriteName::new(asked_for(key));
    let Some(def) = registry.def(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let def = def.clone();
    draft.load_sprite(&name, &def);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_attachment(
    draft: &mut AttachmentDraft,
    registry: &AttachmentRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = AttachmentName::new(asked_for(key));
    let Some(spec) = registry.spec(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let spec = spec.clone();
    draft.load_attachment(&name, &spec);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_weapon(
    draft: &mut WeaponDraft,
    registry: &WeaponRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = WeaponName::new(asked_for(key));
    let Some(spec) = registry.spec(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let spec = spec.clone();
    draft.load_weapon(&name, &spec);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_field(
    draft: &mut FieldDraft,
    registry: &FieldDefRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let wanted = FieldKey::new(asked_for(key));
    let Some(def) = registry.def(&wanted) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let def = def.clone();
    draft.load_field(&wanted, &def);
    KeyLookup::Loaded
}

pub(in crate::net_qa::commands::write::load) fn load_melee_weapon(
    draft: &mut MeleeWeaponDraft,
    registry: &MeleeWeaponRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let name = WeaponName::new(asked_for(key));
    let Some(spec) = registry.spec(&name) else {
        return KeyLookup::NoSuchKey(known_keys(
            registry
                .keys()
                .map(|key| EditorKeyNet::new(key.as_str().to_owned())),
        ));
    };
    let spec = spec.clone();
    draft.load_melee_weapon(&name, &spec);
    KeyLookup::Loaded
}
