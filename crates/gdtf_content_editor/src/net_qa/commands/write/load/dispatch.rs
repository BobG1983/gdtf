//! Pick the family a load runs against, or say why that mode loads nothing.

use super::{
    families::{
        KeyLookup, load_armor, load_attachment, load_gang, load_injury, load_melee_weapon,
        load_sprite, load_weapon,
    },
    theme::load_theme,
};
use crate::{
    EditorMode,
    net_qa::{
        forms::{EditorForms, EditorRegistries},
        wire::{EditorKeyNet, EditorRefusalNet},
    },
};

/// What a load attempt did, before it is put on the wire.
pub(in crate::net_qa::commands::write::load) enum LoadAttempt {
    /// The family answered, either with a hit or with a miss.
    Answered(KeyLookup),
    /// This build loads no draft for that mode.
    Refused(EditorRefusalNet),
    /// That mode's draft or registry is not in the world.
    Missing,
}

/// Run the load for `mode`, calling that family's own `load_*` method.
pub(in crate::net_qa::commands::write::load) fn load_for_mode(
    mode: EditorMode,
    key: &EditorKeyNet,
    forms: &mut EditorForms<'_>,
    registries: &EditorRegistries<'_>,
) -> LoadAttempt {
    match mode {
        EditorMode::Terrain | EditorMode::Prefab => {
            LoadAttempt::Refused(EditorRefusalNet::NoLoadAction)
        }
        EditorMode::Theme => match (
            forms.theme.as_deref_mut(),
            forms.session.as_deref_mut(),
            registries.themes.as_deref(),
        ) {
            (Some(draft), Some(session), Some(registry)) => {
                LoadAttempt::Answered(load_theme(draft, session, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::Gang => match (forms.gang.as_deref_mut(), registries.gangs.as_deref()) {
            (Some(draft), Some(registry)) => LoadAttempt::Answered(load_gang(draft, registry, key)),
            _ => LoadAttempt::Missing,
        },
        EditorMode::Armor => match (forms.armor.as_deref_mut(), registries.armor.as_deref()) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_armor(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::Injury => match (forms.injury.as_deref_mut(), registries.injuries.as_deref()) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_injury(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::Sprite => match (forms.sprite.as_deref_mut(), registries.sprites.as_deref()) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_sprite(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::Attachment => match (
            forms.attachment.as_deref_mut(),
            registries.attachments.as_deref(),
        ) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_attachment(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::Weapon => match (forms.weapon.as_deref_mut(), registries.weapons.as_deref()) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_weapon(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
        EditorMode::MeleeWeapon => match (
            forms.melee_weapon.as_deref_mut(),
            registries.melee_weapon.as_deref(),
        ) {
            (Some(draft), Some(registry)) => {
                LoadAttempt::Answered(load_melee_weapon(draft, registry, key))
            }
            _ => LoadAttempt::Missing,
        },
    }
}
