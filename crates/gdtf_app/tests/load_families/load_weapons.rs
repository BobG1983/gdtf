//! Load weapons into [`WeaponsFamily`] by authored member keys.
//! Value-agnostic: registry presence and hit-type / ammo properties only.
use bevy::app::Startup;
use gdtf_app::test_support::{AppState, app_state, load_released, seed_load_fallbacks};
use gdtf_battle_sim::weapon::{HitType, WeaponRegistry};
use gdtf_content_families::WeaponsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

use super::load_suite;

impl FamilyLoadContract for WeaponsFamily {
    fn is_empty(registry: &WeaponRegistry) -> bool {
        registry.is_empty()
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

    advance_until_resource_exists::<WeaponRegistry>(&mut app);

    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
    }

    advance_until(&mut app, load_released);
}

#[test]
fn shipped_cone_and_line_weapons_resolve_their_aoe_hit_types() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app);

    let registry = app.world().get_resource::<WeaponRegistry>();
    assert!(
        registry.is_some(),
        "the real weapons folder load must insert a WeaponRegistry (last AppState was {:?})",
        app_state(&app),
    );
    let Some(registry) = registry else {
        return;
    };

    let any_cone = registry.iter().any(|(_name, spec)| {
        spec.fire_mode
            .iter()
            .any(|mode| matches!(mode.hit_type, HitType::Cone { .. }))
    });
    assert!(
        any_cone,
        "at least one shipped weapon must offer a Cone hit type (property, not a named stem)",
    );

    let any_line = registry.iter().any(|(_name, spec)| {
        spec.fire_mode
            .iter()
            .any(|mode| matches!(mode.hit_type, HitType::Line { .. }))
    });
    assert!(
        any_line,
        "at least one shipped weapon must offer a Line hit type (property, not a named stem)",
    );
}

#[test]
fn shipped_weapons_each_declare_an_accepted_ammo_type() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<WeaponRegistry>(&mut app);

    let registry = app.world().get_resource::<WeaponRegistry>();
    assert!(
        registry.is_some(),
        "the real weapons folder load must insert a WeaponRegistry (last AppState was {:?})",
        app_state(&app),
    );
    let Some(registry) = registry else {
        return;
    };

    assert!(
        !registry.is_empty(),
        "shipped weapons folder must yield at least one weapon",
    );
    let distinct: std::collections::HashSet<_> = registry.iter().map(|(_n, s)| s.accepts).collect();
    assert!(
        distinct.len() >= 2,
        "shipped weapons should exercise more than one ammo type (property of the catalog)",
    );
}
