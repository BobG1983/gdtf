use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

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
        let Some(stem) = stem else { return };
        registry.insert(ArmorName::new(stem.into_inner()), *spec);
    }
}
