//! Prefab content folder layout constants and the prefab content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::level::{Prefab, PrefabName, PrefabRegistry, PrefabSpec};

/// Root folder for map prefabs.
pub const PREFABS_FOLDER: &str = "content/maps";

/// Extension for prefab files.
pub const PREFAB_EXTENSION: &str = "prefab.ron";

/// The key one prefab is recorded under: its whole registry key plus its file stem.
///
/// The registry keys on theme, size and role, none of which a stem alone names.
#[must_use]
pub fn member_key(stem: &ContentFileStem, spec: &PrefabSpec) -> ContentMemberKey {
    ContentMemberKey::new(format!(
        "{}/{}x{}x{}/{:?}/{}",
        *spec.theme,
        *spec.size.width(),
        *spec.size.height(),
        *spec.size.levels(),
        spec.role,
        **stem,
    ))
}

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
    ) -> Option<ContentMemberKey> {
        let stem = stem?;
        let key = member_key(&stem, spec);
        registry.insert(Prefab::new(
            PrefabName::new(stem.into_inner()),
            spec.clone(),
        ));
        Some(key)
    }
}
