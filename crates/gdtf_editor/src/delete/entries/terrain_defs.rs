//! Rewriting the terrain defs that name a record a delete is removing.

use std::path::Path;

use bevy::prelude::World;
use cobalt_ron_assets::write_ron_pretty;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths};
use gdtf_battle_sim::{
    terrain::def::{LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainSimKind, TerrainUuid},
    weapon::WeaponName,
};
use gdtf_content_families::TerrainDefsFamily;

use crate::delete::{
    offer::{ReplacementCandidate, ReplacementLabel, labelled_candidates},
    resolution::DroppedReferences,
};

/// The key a terrain def's source path and every finding against it are recorded under.
pub(super) fn terrain_member_key(key: TerrainUuid) -> ContentMemberKey {
    ContentMemberKey::new((*key).to_string())
}

/// The terrain UUID a member key names, if it is one.
pub(super) fn terrain_uuid(key: &ContentMemberKey) -> Option<TerrainUuid> {
    bevy::asset::uuid::Uuid::parse_str(key)
        .ok()
        .map(TerrainUuid::new)
}

/// Every terrain def as a replacement row, its display name disambiguated by key.
pub(super) fn terrain_candidates(world: &World) -> Vec<ReplacementCandidate> {
    world
        .get_resource::<TerrainDefRegistry>()
        .map_or_else(Vec::new, |registry| {
            labelled_candidates(registry.defs().map(|(key, def)| {
                (
                    terrain_member_key(*key),
                    ReplacementLabel::new((*def.display_name).clone()),
                )
            }))
        })
}

/// Point every `leaves_behind` naming `deleted` at `replacement` instead.
pub(super) fn replace_leaves_behind(
    world: &mut World,
    root: &Path,
    deleted: TerrainUuid,
    replacement: TerrainUuid,
) -> DroppedReferences {
    rewrite_terrain_defs(world, root, |def| {
        let names_it = def.leaves_behind == LeavesBehind::Piece(deleted);
        if names_it {
            def.leaves_behind = LeavesBehind::Piece(replacement);
        }
        names_it
    })
}

/// Point every emplacement mounting `deleted` at `replacement` instead.
pub(super) fn replace_mounted_weapon(
    world: &mut World,
    root: &Path,
    deleted: &WeaponName,
    replacement: &WeaponName,
) -> DroppedReferences {
    rewrite_terrain_defs(world, root, |def| {
        let TerrainSimKind::Emplacement { mounted_weapon, .. } = &mut def.sim_kind else {
            return false;
        };
        let names_it = mounted_weapon == deleted;
        if names_it {
            *mounted_weapon = replacement.clone();
        }
        names_it
    })
}

// Rewrite every def `edit` changes, each file written before its registry entry.
fn rewrite_terrain_defs(
    world: &mut World,
    root: &Path,
    edit: impl Fn(&mut TerrainDef) -> bool,
) -> DroppedReferences {
    let Some(registry) = world.get_resource::<TerrainDefRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(TerrainUuid, TerrainDef)> = registry
        .defs()
        .filter_map(|(key, def)| {
            let mut next = def.clone();
            edit(&mut next).then_some((*key, next))
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
            .path(&terrain_member_key(*key))
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
