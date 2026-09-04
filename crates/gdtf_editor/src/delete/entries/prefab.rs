//! The prefab delete: nothing names a prefab, so its in-use check always clears.

use gdtf_assets::{ContentFileStem, ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::level::{Prefab, PrefabKey, PrefabRegistry};
use gdtf_content_families::{PrefabsFamily, prefabs::member_key};

use crate::{
    delete::registry::{DeleteEntry, DeleteScreen},
    mode::EditorMode,
};

/// The finding family label every prefab finding carries.
pub(crate) const PREFAB_FAMILY: &str = "PrefabRegistry";

/// The delete for one authored prefab, offered on the Prefab tab.
pub(crate) fn prefab_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(PREFAB_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Prefab, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<PrefabRegistry>()?;
            let (prefab_key, name) = located(&registry, key)?;
            let prefab = registry.remove(&prefab_key, &name)?;
            Some(Box::new(prefab))
        }),
        Box::new(|world, _key, record| {
            let Ok(prefab) = record.downcast::<Prefab>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<PrefabRegistry>() {
                registry.insert(*prefab);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<PrefabsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
}

// The registry key and prefab name the member key names, if the registry holds one.
fn located(
    registry: &PrefabRegistry,
    key: &ContentMemberKey,
) -> Option<(PrefabKey, gdtf_battle_sim::level::PrefabName)> {
    registry.iter().find_map(|prefab| {
        let spec = prefab.spec();
        let stem = ContentFileStem::new((**prefab.name()).clone());
        (*member_key(&stem, spec) == **key).then(|| {
            (
                PrefabKey::new(spec.theme, spec.size, spec.role),
                prefab.name().clone(),
            )
        })
    })
}
