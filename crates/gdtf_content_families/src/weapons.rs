use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

pub struct WeaponsFamily;

impl ContentFamily for WeaponsFamily {
    type Spec = WeaponSpec;
    type Registry = WeaponRegistry;

    const EXTENSION: &'static str = "weapon.ron";
    const FOLDER: &'static str = "content/weapons/ranged";

    fn insert_member(
        registry: &mut WeaponRegistry,
        stem: Option<ContentFileStem>,
        spec: &WeaponSpec,
    ) {
        let Some(stem) = stem else { return };
        registry.insert(WeaponName::new(stem.into_inner()), spec.clone());
    }
}
