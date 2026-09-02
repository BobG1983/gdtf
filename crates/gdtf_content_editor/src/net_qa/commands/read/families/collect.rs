//! Read one family's members off its own registry, with no new registry method.

use core::ops::Deref;

use bevy::asset::uuid::Uuid;

use crate::{
    net_qa::{
        forms::EditorRegistries,
        wire::{EditorFamilyEntryNet, EditorFamilyLabelNet, EditorKeyNet, EditorModeNet},
    },
    terrain_form::load_candidates,
};

// A name-keyed family shows its key as the label; that is what the editor's pickers show.
fn keyed<N: Deref<Target = String>>(name: &N) -> EditorFamilyEntryNet {
    EditorFamilyEntryNet::new(
        EditorKeyNet::new((**name).clone()),
        EditorFamilyLabelNet::new((**name).clone()),
    )
}

// A UUID-keyed family shows the def's display name, the way the theme picker does.
fn titled<K, D>(key: &K, display: &D) -> EditorFamilyEntryNet
where
    K: Deref<Target = Uuid>,
    D: Deref<Target = String>,
{
    EditorFamilyEntryNet::new(
        EditorKeyNet::new((**key).to_string()),
        EditorFamilyLabelNet::new((**display).clone()),
    )
}

/// Every member `family` holds, unordered. A missing registry answers as an empty family.
pub(super) fn entries_of(
    family: EditorModeNet,
    registries: &EditorRegistries<'_>,
) -> Vec<EditorFamilyEntryNet> {
    match family {
        // The Terrain load picker's own rows, so a duplicated display name is one row per def.
        EditorModeNet::Terrain => registries
            .terrain
            .as_deref()
            .map_or_else(Vec::new, |registry| {
                load_candidates(registry)
                    .into_iter()
                    .map(|(key, label)| {
                        EditorFamilyEntryNet::new(
                            EditorKeyNet::new((*key).to_string()),
                            EditorFamilyLabelNet::new(label),
                        )
                    })
                    .collect()
            }),
        EditorModeNet::Theme => registries
            .themes
            .as_deref()
            .map_or_else(Vec::new, |registry| {
                registry
                    .defs()
                    .map(|(key, def)| titled(key, &def.display_name))
                    .collect()
            }),
        EditorModeNet::Gang => registries
            .gangs
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::Armor => registries
            .armor
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::Injury => registries
            .injuries
            .as_deref()
            .map_or_else(Vec::new, |registry| {
                registry.iter().map(|(key, _)| keyed(key)).collect()
            }),
        EditorModeNet::Sprite => registries
            .sprites
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::Attachment => registries
            .attachments
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::Weapon => registries
            .weapons
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::MeleeWeapon => registries
            .melee_weapon
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorModeNet::Field => registries
            .fields
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        // The Prefab tab is the map canvas and owns no registry; `families_to_answer` refuses it.
        EditorModeNet::Prefab => Vec::new(),
    }
}
