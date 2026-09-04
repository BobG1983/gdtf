//! The theme delete: every prefab and the situation take the replacement theme.

use std::path::Path;

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry};
use gdtf_content_families::ThemeDefsFamily;

use super::{
    prefab_specs::replace_prefab_theme,
    situations::replace_situation_theme,
    theme_defs::{theme_candidates, theme_uuid},
};
use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    mode::EditorMode,
};

/// The finding family label every theme reference finding carries.
pub(crate) const THEME_FAMILY: &str = "UuidThemeRegistry";

// The family label the loaded situation's own references are recorded under.
const SITUATION_REFERRER: &str = "LoadedSituation";

// The family label a prefab's own theme reference is recorded under.
const PREFAB_REFERRER: &str = "PrefabRegistry";

/// The delete for one theme def, offered on the Theme tab.
pub(crate) fn theme_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(THEME_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Theme, None),
        Box::new(|world, key| {
            let uuid = theme_uuid(key)?;
            let mut registry = world.get_resource_mut::<UuidThemeRegistry>()?;
            let def = registry.remove(&uuid)?;
            Some(Box::new(def))
        }),
        Box::new(|world, key, record| {
            let Ok(def) = record.downcast::<UuidThemeDef>() else {
                return;
            };
            let Some(uuid) = theme_uuid(key) else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<UuidThemeRegistry>() {
                registry.insert(uuid, *def);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<ThemeDefsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_candidates(Box::new(theme_candidates))
    .replacing_in(
        FindingFamily::new(PREFAB_REFERRER.to_owned()),
        Box::new(|world, key, replacement| in_root(world, key, replacement, replace_prefab_theme)),
    )
    .replacing_in(
        FindingFamily::new(SITUATION_REFERRER.to_owned()),
        Box::new(|world, key, replacement| {
            in_root(world, key, replacement, replace_situation_theme)
        }),
    )
}

// Run one rewrite under the assets root the delete is writing to.
fn in_root(
    world: &mut World,
    key: &ContentMemberKey,
    replacement: &ContentMemberKey,
    rewrite: fn(&mut World, &Path, ThemeUuid, ThemeUuid) -> DroppedReferences,
) -> DroppedReferences {
    let (Some(deleted), Some(chosen)) = (theme_uuid(key), theme_uuid(replacement)) else {
        return DroppedReferences::Failed;
    };
    delete_assets_root(world).map_or(DroppedReferences::Failed, |root| {
        rewrite(world, &root, deleted, chosen)
    })
}
