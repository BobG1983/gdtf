//! Pick the writer a save runs, and refuse a name the mode does not carry.

use std::path::Path;

use super::{
    families::{
        save_armor, save_attachment, save_field, save_gang, save_injury, save_melee_weapon,
        save_sprite, save_weapon,
    },
    map_forms::{save_prefab, save_terrain, save_theme},
};
use crate::{
    EditorMode,
    net_qa::{
        forms::{EditorForms, EditorRegistries},
        wire::{EditorContentNameNet, EditorRefusalNet},
    },
    save_record::SaveOutcome,
};

/// What a save attempt did, before it is put on the wire.
pub(in crate::net_qa::commands::write::save) enum SaveAttempt {
    /// The writer ran and reported this.
    Wrote(SaveOutcome),
    /// That mode's draft or registry is not in the world.
    Missing,
}

/// Only Prefab carries its own name field, so only Prefab reads a name argument.
pub(in crate::net_qa::commands::write::save) fn name_refusal(
    mode: EditorMode,
    name: Option<&EditorContentNameNet>,
) -> Option<EditorRefusalNet> {
    let named = name.is_some();
    if matches!(mode, EditorMode::Prefab) {
        return (!named).then_some(EditorRefusalNet::PrefabNeedsAName);
    }
    named.then_some(EditorRefusalNet::NameBelongsToPrefabOnly)
}

/// Run the save for `mode` under `root`, through that family's root-taking writer.
pub(in crate::net_qa::commands::write::save) fn save_for_mode(
    mode: EditorMode,
    name: Option<&EditorContentNameNet>,
    root: &Path,
    forms: &mut EditorForms<'_>,
    registries: &EditorRegistries<'_>,
) -> SaveAttempt {
    let themes = registries.themes.as_deref();
    match mode {
        EditorMode::Terrain => match (forms.terrain.as_deref_mut(), forms.session.as_deref()) {
            (Some(draft), Some(session)) => {
                SaveAttempt::Wrote(save_terrain(draft, session, themes, root))
            }
            _ => SaveAttempt::Missing,
        },
        EditorMode::Theme => match forms.theme.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_theme(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Prefab => match (
            forms.map.as_deref(),
            forms.session.as_deref(),
            registries.terrain.as_deref(),
            name,
        ) {
            (Some(map), Some(session), Some(terrain), Some(name)) => {
                SaveAttempt::Wrote(save_prefab(map, terrain, session, themes, name, root))
            }
            _ => SaveAttempt::Missing,
        },
        EditorMode::Gang => match forms.gang.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_gang(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Armor => match forms.armor.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_armor(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Injury => match forms.injury.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_injury(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Sprite => match forms.sprite.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_sprite(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Attachment => match forms.attachment.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_attachment(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Weapon => match forms.weapon.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_weapon(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::MeleeWeapon => match forms.melee_weapon.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_melee_weapon(draft, root)),
            None => SaveAttempt::Missing,
        },
        EditorMode::Field => match forms.field.as_deref() {
            Some(draft) => SaveAttempt::Wrote(save_field(draft, root)),
            None => SaveAttempt::Missing,
        },
    }
}
