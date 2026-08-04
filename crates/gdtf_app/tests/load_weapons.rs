//! Load weapons into [`WeaponsFamily`] by authored member keys.
//! Value-agnostic: registry presence and stems only; magnitudes are tuning data.
mod load_suite;

use bevy::app::Startup;
use gdtf_app::test_support::{AppState, app_state, load_released, seed_load_fallbacks};
use gdtf_battle_sim::weapon::{AmmoType, HitType, WeaponName, WeaponRegistry};
use gdtf_content_families::WeaponsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for WeaponsFamily {
    const EXPECTED_MEMBERS: &'static [&'static str] = &["stub_pistol", "las_carbine"];

    fn is_empty(registry: &WeaponRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &WeaponRegistry, label: &str) -> bool {
        registry.spec(&WeaponName::new(label.to_owned())).is_some()
    }
}

#[test]
fn weapons_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<WeaponsFamily>();
}

#[test]
fn load_does_not_leave_without_a_weapon_registry() {
    suite::load_gates_on_registry::<WeaponsFamily>();
}

#[test]
fn real_asset_resolves_weapon_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<WeaponsFamily>();
}

#[test]
fn seeded_startup_does_not_shadow_real_weapon_resolution() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_systems(Startup, seed_load_fallbacks);

    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
        assert!(
            registry
                .spec(&WeaponName::new("stub_pistol".to_owned()))
                .is_some(),
            "the registry must hold `stub_pistol` — the empty seed must NOT have shadowed the resolve",
        );
        assert!(
            registry
                .spec(&WeaponName::new("las_carbine".to_owned()))
                .is_some(),
            "the resolved registry must also hold `las_carbine` (the empty seed held neither)",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with the Startup seed present, Load must still release to Intro (or beyond) once the \
         real weapons resolve; last AppState was {:?}",
        app_state(&app),
    );
}

#[test]
fn shipped_cone_and_line_weapons_resolve_their_aoe_hit_types() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<WeaponRegistry>();
    assert!(
        registry.is_some(),
        "the real weapons folder load must insert a WeaponRegistry within the budget \
         (last AppState was {:?})",
        app_state(&app),
    );
    if let Some(registry) = registry {
        let chem = registry.spec(&WeaponName::new("chem_sprayer".to_owned()));
        assert!(
            chem.is_some(),
            "the shipped chem_sprayer.weapon.ron must resolve through the real folder load",
        );
        if let Some(chem) = chem {
            assert!(
                chem.fire_mode
                    .iter()
                    .any(|mode| matches!(mode.hit_type, HitType::Cone { .. })),
                "the chem_sprayer must offer a fire mode whose HitType deserializes as Cone \
                 (its authored `hit_type: Cone(..)` short-range spray template)",
            );
        }

        let lance = registry.spec(&WeaponName::new("las_lance".to_owned()));
        assert!(
            lance.is_some(),
            "the net-new las_lance.weapon.ron must resolve through the real folder load",
        );
        if let Some(lance) = lance {
            assert!(
                lance
                    .fire_mode
                    .iter()
                    .any(|mode| matches!(mode.hit_type, HitType::Line { .. })),
                "the las_lance must offer a fire mode whose HitType deserializes as Line \
                 (its authored `hit_type: Line(..)` penetrating beam template)",
            );
        }
    }
}

#[test]
fn shipped_weapons_resolve_their_accepted_ammo_types() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<WeaponRegistry>();
    assert!(
        registry.is_some(),
        "the real weapons folder load must insert a WeaponRegistry within the budget \
         (last AppState was {:?})",
        app_state(&app),
    );
    let Some(registry) = registry else {
        return;
    };

    for (stem, expected) in [
        ("stub_pistol", AmmoType::Slug),
        ("las_carbine", AmmoType::Cell),
        ("plasma_pistol", AmmoType::Flask),
        ("chem_sprayer", AmmoType::Canister),
        ("frag_grenade", AmmoType::Grenade),
    ] {
        let spec = registry.spec(&WeaponName::new(stem.to_owned()));
        assert!(
            spec.is_some(),
            "the shipped {stem}.weapon.ron must resolve through the real folder load",
        );
        if let Some(spec) = spec {
            assert_eq!(
                spec.accepts, expected,
                "the shipped {stem} must author `accepts: {expected:?}` (resolved through the loader)",
            );
        }
    }
}
