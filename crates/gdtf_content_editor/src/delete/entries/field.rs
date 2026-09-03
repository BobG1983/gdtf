//! The field delete: the situation and every `on_death` naming it are written back without it.

use std::path::Path;

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily, write_ron_pretty};
use gdtf_battle_sim::{
    effects::{
        fields::{FieldDef, FieldDefRegistry, FieldKey},
        on_death::OnDeathEffect,
    },
    terrain::def::{TerrainDef, TerrainDefRegistry, TerrainUuid},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_content_families::{FieldsFamily, TerrainDefsFamily, situation::LoadedSituation};

use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    mode::EditorMode,
    situation::write_situation_in,
    weapon_form::write_weapon_in,
};

/// The finding family label every field reference finding carries.
pub(crate) const FIELD_FAMILY: &str = "FieldDefRegistry";

/// The delete for one field def, offered on the Field tab.
pub(crate) fn field_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(FIELD_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Field, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<FieldDefRegistry>()?;
            let def = registry.remove(&field_key(key))?;
            Some(Box::new(def))
        }),
        Box::new(|world, key, record| {
            let Ok(def) = record.downcast::<FieldDef>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<FieldDefRegistry>() {
                registry.insert(field_key(key), *def);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<FieldsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_drop(Box::new(|world, key| {
        let field = field_key(key);
        let Some(root) = delete_assets_root(world) else {
            return DroppedReferences::Failed;
        };
        let situation = drop_from_situation(world, &root, &field);
        let terrain = combined(situation, drop_from_terrain_defs(world, &root, &field));
        combined(terrain, drop_from_weapons(world, &root, &field))
    }))
}

// The registry key the member key names.
fn field_key(key: &ContentMemberKey) -> FieldKey {
    FieldKey::new((**key).clone())
}

// A failed part loses to nothing else, and one rewrite is enough to re-check.
const fn combined(left: DroppedReferences, right: DroppedReferences) -> DroppedReferences {
    match (left, right) {
        (DroppedReferences::Failed, _) | (_, DroppedReferences::Failed) => {
            DroppedReferences::Failed
        }
        (DroppedReferences::Rewritten, _) | (_, DroppedReferences::Rewritten) => {
            DroppedReferences::Rewritten
        }
        _ => DroppedReferences::Nothing,
    }
}

// Whether an on-death list leaves this field behind.
fn leaves_field(effects: &[OnDeathEffect], field: &FieldKey) -> bool {
    effects.iter().any(|effect| is_leave_field(effect, field))
}

// Whether one effect leaves this field behind.
fn is_leave_field(effect: &OnDeathEffect, field: &FieldKey) -> bool {
    match effect {
        OnDeathEffect::LeaveField { field: left } => left == field,
        OnDeathEffect::Explode { .. } => false,
    }
}

// The on-death list without the whole `LeaveField`, which has no field-less form.
fn without_leave_field(effects: &[OnDeathEffect], field: &FieldKey) -> Vec<OnDeathEffect> {
    effects
        .iter()
        .filter(|effect| !is_leave_field(effect, field))
        .cloned()
        .collect()
}

// Take the spawn out of the loaded situation, writing its file before the resource.
fn drop_from_situation(world: &mut World, root: &Path, field: &FieldKey) -> DroppedReferences {
    let Some(loaded) = world.get_resource::<LoadedSituation>() else {
        return DroppedReferences::Nothing;
    };
    if !loaded.fields.iter().any(|spawn| spawn.field == *field) {
        return DroppedReferences::Nothing;
    }
    let mut situation = (**loaded).clone();
    situation.fields.retain(|spawn| spawn.field != *field);
    if write_situation_in(root, &situation).is_err() {
        return DroppedReferences::Failed;
    }
    let Some(mut loaded) = world.get_resource_mut::<LoadedSituation>() else {
        return DroppedReferences::Failed;
    };
    loaded
        .situation_mut()
        .fields
        .retain(|spawn| spawn.field != *field);
    DroppedReferences::Rewritten
}

// Rewrite every terrain def leaving the field behind, each to the file it was read from.
fn drop_from_terrain_defs(world: &mut World, root: &Path, field: &FieldKey) -> DroppedReferences {
    let Some(registry) = world.get_resource::<TerrainDefRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(TerrainUuid, TerrainDef)> = registry
        .defs()
        .filter(|(_key, def)| leaves_field(&def.on_death, field))
        .map(|(key, def)| {
            let mut next = def.clone();
            next.on_death = without_leave_field(&def.on_death, field);
            (*key, next)
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    let Some(sources) = world.get_resource::<ContentSourcePaths<TerrainDefsFamily>>() else {
        return DroppedReferences::Failed;
    };
    let written = rewritten.iter().all(|(key, def)| {
        sources
            .path(&ContentMemberKey::new((**key).to_string()))
            .is_some_and(|relative| write_ron_pretty(&root.join(&**relative), def).is_ok())
    });
    if !written {
        return DroppedReferences::Failed;
    }
    let Some(mut registry) = world.get_resource_mut::<TerrainDefRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (key, def) in rewritten {
        registry.insert(key, def);
    }
    DroppedReferences::Rewritten
}

// Rewrite every ranged weapon spec leaving the field behind, writing each file first.
fn drop_from_weapons(world: &mut World, root: &Path, field: &FieldKey) -> DroppedReferences {
    let Some(registry) = world.get_resource::<WeaponRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(WeaponName, WeaponSpec)> = registry
        .iter()
        .filter(|(_key, spec)| leaves_field(&spec.on_death, field))
        .map(|(key, spec)| {
            let mut next = spec.clone();
            next.on_death = without_leave_field(&spec.on_death, field);
            (key.clone(), next)
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    for (key, spec) in &rewritten {
        if write_weapon_in(root, key, spec).is_err() {
            return DroppedReferences::Failed;
        }
    }
    let Some(mut registry) = world.get_resource_mut::<WeaponRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (key, spec) in rewritten {
        registry.insert(key, spec);
    }
    DroppedReferences::Rewritten
}
