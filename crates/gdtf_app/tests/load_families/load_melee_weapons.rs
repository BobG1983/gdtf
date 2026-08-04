//! Load melee weapons into [`MeleeWeaponsFamily`].
//! Value-agnostic: registry presence only (no pinned stems).

use super::load_suite;
use gdtf_battle_sim::weapon::MeleeWeaponRegistry;
use gdtf_content_families::MeleeWeaponsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for MeleeWeaponsFamily {
    fn is_empty(registry: &MeleeWeaponRegistry) -> bool {
        registry.is_empty()
    }
}

#[test]
fn melee_weapons_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<MeleeWeaponsFamily>();
}

#[test]
fn load_does_not_leave_without_a_melee_weapon_registry() {
    suite::load_gates_on_registry::<MeleeWeaponsFamily>();
}

#[test]
fn real_asset_resolves_melee_weapon_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<MeleeWeaponsFamily>();
}
