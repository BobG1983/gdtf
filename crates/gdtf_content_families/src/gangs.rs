//! The gangs content family (GTW-415, generic seam since GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

/// The gangs family: `assets/content/gangs/*.gang.ron` → the name-keyed
/// [`GangRegistry`] the battle setup resolves every placed ganger's
/// `(gang, member)` ref against — without it every real battle fails closed
/// with `GangNotFound`.
///
/// STEM-KEYED: `rust_dogs.gang.ron` keys `rust_dogs` (the [`GangName`] a
/// placement references).
pub struct GangsFamily;

impl ContentFamily for GangsFamily {
    type Spec = GangRoster;
    type Registry = GangRegistry;

    const EXTENSION: &'static str = "gang.ron";
    const FOLDER: &'static str = "content/gangs";

    fn insert_member(
        registry: &mut GangRegistry,
        stem: Option<ContentFileStem>,
        roster: &GangRoster,
    ) {
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(GangName::new(stem.into_inner()), roster.clone());
    }
}
