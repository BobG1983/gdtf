//! The UUID-keyed theme-defs content family (GTW-487, generic machinery since
//! GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::level::{UuidThemeDef, UuidThemeRegistry};

use crate::TerrainDefsFamily;

/// The theme-defs family:
/// `assets/content/terrain/<theme>/*.terrain_theme.ron` → the UUID-keyed
/// [`UuidThemeRegistry`] the procgen theme resolution consumes.
///
/// PAYLOAD-KEYED: each def carries its OWN
/// [`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid) inside
/// ([`UuidThemeDef::key`]) — the filename is irrelevant to the key. Shares the
/// MIXED `content/terrain/` tree with
/// [`TerrainDefsFamily`](crate::TerrainDefsFamily): the generic walk's
/// unconditional `TypeId` filter skips the terrain-def members here (and
/// vice-versa).
pub struct ThemeDefsFamily;

impl ContentFamily for ThemeDefsFamily {
    type Spec = UuidThemeDef;
    type Registry = UuidThemeRegistry;

    const EXTENSION: &'static str = "terrain_theme.ron";
    // GTW-634 A1: the mixed tree's root has ONE owning spelling — the theme
    // defs walk the SAME folder as the terrain defs, so this is DERIVED from
    // `TerrainDefsFamily::FOLDER`, never a second hand-maintained literal.
    const FOLDER: &'static str = TerrainDefsFamily::FOLDER;

    fn insert_member(
        registry: &mut UuidThemeRegistry,
        _stem: Option<ContentFileStem>,
        def: &UuidThemeDef,
    ) {
        // Payload-keyed: the def's own UUID is the key; the stem is unused.
        registry.insert(def.key, def.clone());
    }
}
