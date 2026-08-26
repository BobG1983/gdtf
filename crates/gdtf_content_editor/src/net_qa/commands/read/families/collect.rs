//! Read one family's members off its own registry, with no new registry method.

use core::ops::Deref;

use bevy::asset::uuid::Uuid;

use crate::net_qa::{
    forms::EditorRegistries,
    wire::{EditorFamilyEntryNet, EditorFamilyLabelNet, EditorFamilyNet, EditorKeyNet},
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
    family: EditorFamilyNet,
    registries: &EditorRegistries<'_>,
) -> Vec<EditorFamilyEntryNet> {
    match family {
        EditorFamilyNet::Terrain => {
            registries
                .terrain
                .as_deref()
                .map_or_else(Vec::new, |registry| {
                    registry
                        .defs()
                        .map(|(key, def)| titled(key, &def.display_name))
                        .collect()
                })
        }
        EditorFamilyNet::Theme => registries
            .themes
            .as_deref()
            .map_or_else(Vec::new, |registry| {
                registry
                    .defs()
                    .map(|(key, def)| titled(key, &def.display_name))
                    .collect()
            }),
        EditorFamilyNet::Gang => registries
            .gangs
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::Armor => registries
            .armor
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::Injury => registries
            .injuries
            .as_deref()
            .map_or_else(Vec::new, |registry| {
                registry.iter().map(|(key, _)| keyed(key)).collect()
            }),
        EditorFamilyNet::Sprite => registries
            .sprites
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::Attachment => registries
            .attachments
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::Weapon => registries
            .weapons
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::MeleeWeapon => registries
            .melee_weapon
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
        EditorFamilyNet::Field => registries
            .fields
            .as_deref()
            .map_or_else(Vec::new, |registry| registry.keys().map(keyed).collect()),
    }
}
