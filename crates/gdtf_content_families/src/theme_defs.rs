//! The UUID-keyed theme-defs content family (GTW-487, generic seam since
//! GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::level::{UuidThemeDef, UuidThemeRegistry};

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
    const FOLDER: &'static str = "content/terrain";

    fn insert_member(
        registry: &mut UuidThemeRegistry,
        _stem: Option<ContentFileStem>,
        def: &UuidThemeDef,
    ) {
        // Payload-keyed: the def's own UUID is the key; the stem is unused.
        registry.insert(def.key, def.clone());
    }
}
