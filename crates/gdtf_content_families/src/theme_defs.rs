//! Theme definition content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::level::{UuidThemeDef, UuidThemeRegistry};

use crate::TerrainDefsFamily;

/// Loads `*.terrain_theme.ron` files into [`UuidThemeRegistry`].
pub struct ThemeDefsFamily;

impl ContentFamily for ThemeDefsFamily {
    type Spec = UuidThemeDef;
    type Registry = UuidThemeRegistry;

    const EXTENSION: &'static str = "terrain_theme.ron";
    const FOLDER: &'static str = TerrainDefsFamily::FOLDER;

    fn insert_member(
        registry: &mut UuidThemeRegistry,
        _stem: Option<ContentFileStem>,
        def: &UuidThemeDef,
    ) -> Option<ContentMemberKey> {
        registry.insert(def.key, def.clone());
        Some(ContentMemberKey::new((*def.key).to_string()))
    }
}
