//! Rewriting the authored prefabs that name a record a delete is removing.

use std::path::Path;

use bevy::prelude::World;
use cobalt_ron_assets::write_ron_pretty;
use gdtf_assets::{ContentFileStem, ContentMemberKey, ContentSourcePaths};
use gdtf_battle_sim::{
    level::{Prefab, PrefabKey, PrefabName, PrefabRegistry, PrefabSpec, ThemeUuid},
    terrain::def::TerrainUuid,
};
use gdtf_content_families::{PrefabsFamily, prefabs::member_key};

use crate::delete::resolution::DroppedReferences;

// One prefab as the rewrite holds it: its name, the key it was found at, and its new spec.
struct RewrittenPrefab {
    name: PrefabName,
    was:  PrefabKey,
    spec: PrefabSpec,
}

impl RewrittenPrefab {
    // The member key this prefab was recorded under before the rewrite.
    fn old_key(&self) -> ContentMemberKey {
        member_key(&ContentFileStem::new((*self.name).clone()), &was_spec(self))
    }

    // The member key this prefab is recorded under after the rewrite.
    fn new_key(&self) -> ContentMemberKey {
        member_key(&ContentFileStem::new((*self.name).clone()), &self.spec)
    }
}

// The spec as the registry held it, which only the prefab's own key still names.
fn was_spec(prefab: &RewrittenPrefab) -> PrefabSpec {
    PrefabSpec::new(
        prefab.was.theme,
        prefab.was.size,
        prefab.was.role,
        prefab.spec.placements.clone(),
    )
}

/// Point every prefab placement naming `deleted` at `replacement` instead.
pub(super) fn replace_prefab_piece(
    world: &mut World,
    root: &Path,
    deleted: TerrainUuid,
    replacement: TerrainUuid,
) -> DroppedReferences {
    rewrite_prefabs(world, root, |spec| {
        let mut changed = false;
        for placement in &mut spec.placements {
            if placement.piece == deleted {
                placement.piece = replacement;
                changed = true;
            }
        }
        changed
    })
}

/// Point every prefab belonging to `deleted` at the `replacement` theme instead.
pub(super) fn replace_prefab_theme(
    world: &mut World,
    root: &Path,
    deleted: ThemeUuid,
    replacement: ThemeUuid,
) -> DroppedReferences {
    rewrite_prefabs(world, root, |spec| {
        let names_it = spec.theme == deleted;
        if names_it {
            spec.theme = replacement;
        }
        names_it
    })
}

// Rewrite every prefab `edit` changes, each written to the file it was read from.
fn rewrite_prefabs(
    world: &mut World,
    root: &Path,
    edit: impl Fn(&mut PrefabSpec) -> bool,
) -> DroppedReferences {
    let Some(registry) = world.get_resource::<PrefabRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<RewrittenPrefab> = registry
        .iter()
        .filter_map(|prefab| {
            let mut spec = prefab.spec().clone();
            edit(&mut spec).then(|| RewrittenPrefab {
                name: prefab.name().clone(),
                was: PrefabKey::new(prefab.spec().theme, prefab.spec().size, prefab.spec().role),
                spec,
            })
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    if !write_prefabs(world, root, &rewritten) {
        return DroppedReferences::Failed;
    }
    if !reseat_prefabs(world, &rewritten) {
        return DroppedReferences::Failed;
    }
    move_source_paths(world, &rewritten);
    DroppedReferences::Rewritten
}

// Write each rewritten prefab to the path its old key still records.
fn write_prefabs(world: &World, root: &Path, rewritten: &[RewrittenPrefab]) -> bool {
    let Some(sources) = world.get_resource::<ContentSourcePaths<PrefabsFamily>>() else {
        return false;
    };
    rewritten.iter().all(|prefab| {
        sources
            .path(&prefab.old_key())
            .is_some_and(|relative| write_ron_pretty(&root.join(&**relative), &prefab.spec).is_ok())
    })
}

// Take each prefab out of the bucket it was in and put the rewritten spec in its own.
fn reseat_prefabs(world: &mut World, rewritten: &[RewrittenPrefab]) -> bool {
    let Some(mut registry) = world.get_resource_mut::<PrefabRegistry>() else {
        return false;
    };
    for prefab in rewritten {
        registry.remove(&prefab.was, &prefab.name);
        registry.insert(Prefab::new(prefab.name.clone(), prefab.spec.clone()));
    }
    true
}

// Follow each prefab's recorded file to the key the rewritten spec is now held at.
fn move_source_paths(world: &mut World, rewritten: &[RewrittenPrefab]) {
    let Some(mut sources) = world.get_resource_mut::<ContentSourcePaths<PrefabsFamily>>() else {
        return;
    };
    for prefab in rewritten {
        sources.rekey(&prefab.old_key(), prefab.new_key());
    }
}
