//! The UUID-keyed terrain-defs content family (GTW-487, generic machinery since
//! GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry};

/// The terrain-defs family: `assets/content/terrain/<theme>/*.terrain_def.ron`
/// → the UUID-keyed [`TerrainDefRegistry`] the sim + procgen + presenter
/// consume (GTW-491/492/493).
///
/// PAYLOAD-KEYED: each def carries its OWN
/// [`TerrainUuid`](gdtf_battle_sim::terrain::def::TerrainUuid) inside
/// ([`TerrainDef::key`]) — the filename is irrelevant to the key. The
/// `content/terrain/` tree is a MIXED folder shared with
/// [`ThemeDefsFamily`](crate::ThemeDefsFamily): the generic walk's
/// unconditional `TypeId` filter skips the theme members here (and vice-versa).
pub struct TerrainDefsFamily;

impl ContentFamily for TerrainDefsFamily {
    type Spec = TerrainDef;
    type Registry = TerrainDefRegistry;

    const EXTENSION: &'static str = "terrain_def.ron";
    const FOLDER: &'static str = "content/terrain";

    fn insert_member(
        registry: &mut TerrainDefRegistry,
        _stem: Option<ContentFileStem>,
        def: &TerrainDef,
    ) {
        // Payload-keyed: the def's own UUID is the key; the stem is unused.
        registry.insert(def.key, def.clone());
    }
}
