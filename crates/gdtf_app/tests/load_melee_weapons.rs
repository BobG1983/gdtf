//! GTW-505 / GTW-580: the MELEE-weapons family's load coverage — the thin
//! wrapper over the generic per-family suite (`load_suite::suite`).
//!
//! New coverage with GTW-580: the melee family was GTW-570-migrated onto the
//! generic content-family seam but never had its own load test — the generic
//! suite gives it the same tier structure as every other folder family for
//! the cost of this wrapper. VALUE-AGNOSTIC: registry presence + the authored
//! filename-stem keys (`fists` is load-bearing — it is the default melee
//! resolve every ganger without an authored melee weapon falls to).

mod load_suite;

use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for MeleeWeaponsFamily {
    /// The shipped stems: `fists.melee_weapon.ron` (the setup default) and
    /// `chainsword.melee_weapon.ron` (the GTW-554 attachment-fitted melee).
    const EXPECTED_MEMBERS: &'static [&'static str] = &["fists", "chainsword"];

    fn is_empty(registry: &MeleeWeaponRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &MeleeWeaponRegistry, label: &str) -> bool {
        registry.spec(&WeaponName::new(label.to_owned())).is_some()
    }
}

/// Tier (a) — the melee-weapons loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn melee_weapons_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<MeleeWeaponsFamily>();
}

/// Tier (a) companion — the Load→Intro transition GATES on the
/// [`MeleeWeaponRegistry`] (the GTW-505 gate clause: the melee folder is
/// verified loaded before any battle arms melee).
#[test]
fn load_does_not_leave_without_a_melee_weapon_registry() {
    suite::load_gates_on_registry::<MeleeWeaponsFamily>();
}

/// Tier (b) — the REAL `assets/content/weapons/melee/` folder resolves into a
/// stem-keyed [`MeleeWeaponRegistry`] through the Load code path.
#[test]
fn real_asset_resolves_melee_weapon_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<MeleeWeaponsFamily>();
}
