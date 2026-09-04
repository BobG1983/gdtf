//! The field delete: the situation and every `on_death` naming it are written back without it.

use std::path::Path;

use bevy::prelude::World;
use cobalt_ron_assets::write_ron_pretty;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
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

// The family label the loaded situation's own references are recorded under.
const SITUATION_REFERRER: &str = "LoadedSituation";

// The family label a terrain def's own on-death reference is recorded under.
const TERRAIN_REFERRER: &str = "TerrainDefRegistry";

// The family label a ranged weapon's own on-death reference is recorded under.
const WEAPON_REFERRER: &str = "WeaponRegistry";

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
    .dropping_from(
        FindingFamily::new(SITUATION_REFERRER.to_owned()),
        Box::new(|world, key| in_root(world, key, drop_from_situation)),
    )
    .dropping_from(
        FindingFamily::new(TERRAIN_REFERRER.to_owned()),
        Box::new(|world, key| in_root(world, key, drop_from_terrain_defs)),
    )
    .dropping_from(
        FindingFamily::new(WEAPON_REFERRER.to_owned()),
        Box::new(|world, key| in_root(world, key, drop_from_weapons)),
    )
}

// Run one half of the field drop under the assets root the delete is writing to.
fn in_root(
    world: &mut World,
    key: &ContentMemberKey,
    drop: fn(&mut World, &Path, &FieldKey) -> DroppedReferences,
) -> DroppedReferences {
    let field = field_key(key);
    delete_assets_root(world).map_or(DroppedReferences::Failed, |root| drop(world, &root, &field))
}

// The registry key the member key names.
fn field_key(key: &ContentMemberKey) -> FieldKey {
    FieldKey::new((**key).clone())
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
