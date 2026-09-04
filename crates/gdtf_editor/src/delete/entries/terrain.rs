//! The terrain delete: every record naming the piece takes the replacement instead.

use std::path::Path;

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::TerrainDefsFamily;

use super::{
    prefab_specs::replace_prefab_piece,
    situations::replace_situation_piece,
    terrain_defs::{replace_leaves_behind, terrain_candidates, terrain_uuid},
    theme_defs::replace_theme_piece,
};
use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    mode::EditorMode,
};

/// The finding family label every terrain piece reference finding carries.
pub(crate) const TERRAIN_FAMILY: &str = "TerrainDefRegistry";

// The family label the loaded situation's own references are recorded under.
const SITUATION_REFERRER: &str = "LoadedSituation";

// The family label a theme def's own piece references are recorded under.
const THEME_REFERRER: &str = "UuidThemeRegistry";

// The family label a prefab's own placement references are recorded under.
const PREFAB_REFERRER: &str = "PrefabRegistry";

/// The delete for one terrain def, offered on the Terrain tab.
pub(crate) fn terrain_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(TERRAIN_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Terrain, None),
        Box::new(|world, key| {
            let uuid = terrain_uuid(key)?;
            let mut registry = world.get_resource_mut::<TerrainDefRegistry>()?;
            let def = registry.remove(&uuid)?;
            Some(Box::new(def))
        }),
        Box::new(|world, key, record| {
            let Ok(def) = record.downcast::<TerrainDef>() else {
                return;
            };
            let Some(uuid) = terrain_uuid(key) else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<TerrainDefRegistry>() {
                registry.insert(uuid, *def);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<TerrainDefsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_candidates(Box::new(terrain_candidates))
    .replacing_in(
        FindingFamily::new(THEME_REFERRER.to_owned()),
        Box::new(|world, key, replacement| in_root(world, key, replacement, replace_theme_piece)),
    )
    .replacing_in(
        FindingFamily::new(PREFAB_REFERRER.to_owned()),
        Box::new(|world, key, replacement| in_root(world, key, replacement, replace_prefab_piece)),
    )
    .replacing_in(
        FindingFamily::new(SITUATION_REFERRER.to_owned()),
        Box::new(|world, key, replacement| {
            in_root(world, key, replacement, replace_situation_piece)
        }),
    )
    .replacing_in(
        FindingFamily::new(TERRAIN_FAMILY.to_owned()),
        Box::new(|world, key, replacement| in_root(world, key, replacement, replace_leaves_behind)),
    )
}

// Run one rewrite under the assets root the delete is writing to.
fn in_root(
    world: &mut World,
    key: &ContentMemberKey,
    replacement: &ContentMemberKey,
    rewrite: fn(&mut World, &Path, TerrainUuid, TerrainUuid) -> DroppedReferences,
) -> DroppedReferences {
    let (Some(deleted), Some(chosen)) = (terrain_uuid(key), terrain_uuid(replacement)) else {
        return DroppedReferences::Failed;
    };
    delete_assets_root(world).map_or(DroppedReferences::Failed, |root| {
        rewrite(world, &root, deleted, chosen)
    })
}
