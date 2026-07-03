//! The MELEE-weapons content family (GTW-505, generic seam since GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName};

/// The MELEE-weapons family: `assets/content/weapons/melee/*.melee_weapon.ron`
/// → the name-keyed [`MeleeWeaponRegistry`] the setup resolves each ganger's
/// melee weapon against (an authored key or the `fists` default).
///
/// STEM-KEYED: `fists.melee_weapon.ron` keys `fists` (melee weapons share the
/// [`WeaponName`] key type with the ranged family). Its OWN leaf folder
/// (sibling of `ranged/`) so the recursive folder walk sees
/// `.melee_weapon.ron` members only.
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
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(WeaponName::new(stem.into_inner()), spec.clone());
    }
}
