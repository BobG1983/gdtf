//! GTW-257 / GTW-580: the RANGED-weapons family's load coverage — the thin
//! wrapper over the generic per-family suite (`load_suite::suite`), plus the
//! family-bespoke GTW-297 `AC3b` seed-shadow regression pin.
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`WeaponsFamily`] with the authored member keys. The
//! assertions stay VALUE-AGNOSTIC (registry presence + authored filename-stem
//! keys) — the authored weapon magnitudes are tuning DATA, never pinned (the
//! brittle-test rule). The field-to-bundle conversion MECHANISM is covered by
//! the fixture-based sim round-trip
//! (`weapon::test::weapon_spec_round_trips_and_into_bundle_groups_faithfully`).

mod load_suite;

use bevy::app::Startup;
use gdtf_app::test_support::{AppState, app_state, load_released, seed_load_fallbacks};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry};
use gdtf_content_families::WeaponsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a safety net against a genuine never-resolve
/// hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for WeaponsFamily {
    /// The canonical shipped stems: `stub_pistol.weapon.ron` / `las_carbine.weapon.ron`.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["stub_pistol", "las_carbine"];

    fn is_empty(registry: &WeaponRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &WeaponRegistry, label: &str) -> bool {
        registry.spec(&WeaponName::new(label.to_owned())).is_some()
    }
}

/// AC4 (tier a) — the weapons loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn weapons_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<WeaponsFamily>();
}

/// AC4 (companion, tier a) — the Load→Intro transition GATES on the
/// [`WeaponRegistry`] (the GTW-257 gate clause: a battle never starts
/// weapon-less).
#[test]
fn load_does_not_leave_without_a_weapon_registry() {
    suite::load_gates_on_registry::<WeaponsFamily>();
}

/// AC4 (tier b) / GTW-270 — the REAL `assets/content/weapons/ranged/` folder
/// resolves into a stem-keyed [`WeaponRegistry`] through the Load code path.
#[test]
fn real_asset_resolves_weapon_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<WeaponsFamily>();
}

/// GTW-297 (`AC3b`) — the REAL auto-battle path (the affordance itself was
/// retired GTW-749; this test reproduces its Startup seed directly, below): the
/// Load-owned `seed_load_fallbacks` runs AND a real `AssetServer` is present (the
/// GUI auto-battle launch's precondition). NO empty
/// `WeaponRegistry::default()` may shadow the real folder resolve: with an
/// `AssetServer` present neither the bespoke seed nor the GTW-629 seam rider (the
/// registry's fallback now lives on its `register_content_family` line) may insert
/// an empty registry, so the generic content-family resolve (which only runs while
/// the registry is ABSENT) populates it from `assets/content/weapons/ranged/*.weapon.ron`.
///
/// This reproduces the bug's exact preconditions on the real code path: a
/// `GdtfLoadTestAppBuilder` app (live `AssetServer` rooted at the workspace `assets/`)
/// with the genuine `seed_load_fallbacks` system registered on `Startup`, exactly as
/// the (now-retired) `AutoBattlePlugin::build` used to wire it. The assertions
/// encode the fix:
///
/// - the resolved `WeaponRegistry` is non-empty and resolves the canonical `stub_pistol`
///   stem (so battle setup's `weapons.spec("stub_pistol")` would NOT return `None` /
///   `WeaponNotFound`);
/// - the machine reaches `Intro` with the registry present (the gate waited for the
///   REAL registry, not the empty seed).
///
/// PIN: if ANY seed path — a revived seam-family arm, or a seam rider gone
/// unconditional — inserts `WeaponRegistry::default()` while a server is present,
/// the registry exists early, the absence-gated resolve SKIPS the folder, the
/// registry stays empty, `spec("stub_pistol")` returns `None`, and this test goes
/// red — exactly the `AC3b` black-screen failure. Family-bespoke: NEVER genericized
/// away (GTW-580 P9).
#[test]
fn seeded_startup_does_not_shadow_real_weapon_resolution() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    // Wire the REAL auto-battle Startup seed (the path the now-retired
    // AutoBattlePlugin used to register), minus the unrelated `drive_past_menu` that
    // needs the RunningState machinery.
    app.add_systems(Startup, seed_load_fallbacks);

    // Signal-poll the WeaponRegistry insert (not a fixed frame count): the real resolve
    // only inserts the registry ONCE it is fully built from the folder (it stays ABSENT
    // while members are still resolving), so on the GOOD path existence implies the
    // authored weapons are present. The cap is a safety net (GTW-305). The seed-shadow
    // regression instead inserts an EMPTY registry at Startup, which the assertions
    // below catch immediately.
    advance_until_resource_exists::<WeaponRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<WeaponRegistry>() {
        // AC3b pin: with the AssetServer present the empty seed must NOT win — the real
        // folder resolve must populate a registry that holds the authored weapons.
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

    // The gate waited for the REAL registry: the machine releases past Load with it
    // present (Intro is TRANSIENT — probe via `load_released`, GTW-589/GTW-601), so a
    // real auto-battle run would advance toward BattleRunning rather than aborting
    // at Generation with WeaponNotFound.
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with the Startup seed present, Load must still release to Intro (or beyond) once the \
         real weapons resolve; last AppState was {:?}",
        app_state(&app),
    );
}
