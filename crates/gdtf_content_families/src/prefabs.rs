//! Prefab content folder layout constants and the prefab content family.

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::level::{Prefab, PrefabName, PrefabRegistry, PrefabSpec};

/// Root folder for map prefabs.
pub const PREFABS_FOLDER: &str = "content/maps";

/// Extension for prefab files.
pub const PREFAB_EXTENSION: &str = "prefab.ron";

/// Loads `*.prefab.ron` files under the nested map tree into [`PrefabRegistry`].
pub struct PrefabsFamily;

impl ContentFamily for PrefabsFamily {
    type Spec = PrefabSpec;
    type Registry = PrefabRegistry;

    const EXTENSION: &'static str = PREFAB_EXTENSION;
    const FOLDER: &'static str = PREFABS_FOLDER;

    fn insert_member(
        registry: &mut PrefabRegistry,
        stem: Option<ContentFileStem>,
        spec: &PrefabSpec,
    ) {
        let Some(stem) = stem else { return };
        registry.insert(Prefab::new(
            PrefabName::new(stem.into_inner()),
            spec.clone(),
        ));
    }
}
