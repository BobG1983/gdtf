//! the cost of this wrapper. VALUE-AGNOSTIC: registry presence + the authored
//! resolve every ganger without an authored melee weapon falls to).

mod load_suite;

use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for MeleeWeaponsFamily {
            const EXPECTED_MEMBERS: &'static [&'static str] = &["fists", "chainsword"];

    fn is_empty(registry: &MeleeWeaponRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &MeleeWeaponRegistry, label: &str) -> bool {
        registry.spec(&WeaponName::new(label.to_owned())).is_some()
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
