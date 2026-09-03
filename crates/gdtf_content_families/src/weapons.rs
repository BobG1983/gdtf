//! Ranged weapons content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

/// Loads `*.weapon.ron` files into [`WeaponRegistry`].
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
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(WeaponName::new(key.clone()), spec.clone());
        Some(ContentMemberKey::new(key))
    }
}
