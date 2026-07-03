//! The armor content family (GTW-269, generic seam since GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

/// The armor family: `assets/content/armor/*.armor.ron` → the name-keyed
/// [`ArmorRegistry`] the battle setup resolves ganger armor keys against.
///
/// STEM-KEYED: `flak_vest.armor.ron` keys `flak_vest` (the [`ArmorName`] a
/// spawn references).
pub struct ArmorFamily;

impl ContentFamily for ArmorFamily {
    type Spec = ArmorSpec;
    type Registry = ArmorRegistry;

    const EXTENSION: &'static str = "armor.ron";
    const FOLDER: &'static str = "content/armor";

    fn insert_member(
        registry: &mut ArmorRegistry,
        stem: Option<ContentFileStem>,
        spec: &ArmorSpec,
    ) {
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(ArmorName::new(stem.into_inner()), *spec);
    }
}
