use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
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
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(WeaponName::new(key.clone()), spec.clone());
        Some(ContentMemberKey::new(key))
    }
}
