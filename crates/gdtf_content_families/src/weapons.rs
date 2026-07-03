//! The RANGED-weapons content family (GTW-257, generic seam since GTW-570).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

/// The RANGED-weapons family: `assets/content/weapons/ranged/*.weapon.ron` →
/// the name-keyed [`WeaponRegistry`] the battle setup resolves
/// [`GangerSpawn`](gdtf_battle_sim::situation::GangerSpawn) weapon keys
/// against.
///
/// STEM-KEYED: `stub_pistol.weapon.ron` keys `stub_pistol` (the [`WeaponName`]
/// a spawn references). Its OWN leaf folder (the sibling `melee/` folder is a
/// separate family) so the recursive folder walk sees `.weapon.ron` members
/// only.
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
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(WeaponName::new(stem.into_inner()), spec.clone());
    }
}
