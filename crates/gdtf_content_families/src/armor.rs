//! Armor content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

/// Loads `*.armor.ron` files into [`ArmorRegistry`].
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
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(ArmorName::new(key.clone()), *spec);
        Some(ContentMemberKey::new(key))
    }
}
