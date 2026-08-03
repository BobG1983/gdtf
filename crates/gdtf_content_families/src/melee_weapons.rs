use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName};

/// melee weapon against (an authored key or the `fists` default).
pub struct MeleeWeaponsFamily;

impl ContentFamily for MeleeWeaponsFamily {
    type Spec = MeleeWeaponSpec;
    type Registry = MeleeWeaponRegistry;

    const EXTENSION: &'static str = "melee_weapon.ron";
    const FOLDER: &'static str = "content/weapons/melee";

    fn insert_member(
        registry: &mut MeleeWeaponRegistry,
        stem: Option<ContentFileStem>,
        spec: &MeleeWeaponSpec,
    ) {
        let Some(stem) = stem else { return };
        registry.insert(WeaponName::new(stem.into_inner()), spec.clone());
    }
}
