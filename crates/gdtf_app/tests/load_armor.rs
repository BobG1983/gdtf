//! GTW-269 / GTW-580: the armor family's load coverage — the thin wrapper over
//! the generic per-family suite (`load_suite::suite`), plus the family-bespoke
//! seed-shadow regression pin (the GTW-297 `AC3b` class, armor mirror).
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`ArmorFamily`] with the authored member keys. The
//! assertions stay VALUE-AGNOSTIC (registry presence + authored filename-stem
//! keys) — the authored armor magnitudes are tuning DATA, never pinned (the
//! brittle-test rule). The field-to-slot conversion MECHANISM is covered by
//! the fixture-based sim round-trip
//! (`armor::test::armor_spec_round_trips_and_registry_resolves_by_name`).

mod load_suite;

use bevy::app::Startup;
use gdtf_app::test_support::{AppState, app_state, load_released, seed_load_fallbacks};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry};
use gdtf_content_families::ArmorFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a safety net against a genuine never-resolve
/// hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

impl FamilyLoadContract for ArmorFamily {
    /// The canonical shipped stems: `flak_vest.armor.ron` / `carapace_plate.armor.ron`.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["flak_vest", "carapace_plate"];

    fn is_empty(registry: &ArmorRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &ArmorRegistry, label: &str) -> bool {
        registry.spec(&ArmorName::new(label.to_owned())).is_some()
    }
}

/// AC (tier a) — the armor loader registration + folder kick-off no-op cleanly
/// under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn armor_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<ArmorFamily>();
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the
/// [`ArmorRegistry`] (the GTW-269 gate clause: the armor folder is verified
/// loaded before `Load` exits).
#[test]
fn load_does_not_leave_without_an_armor_registry() {
    suite::load_gates_on_registry::<ArmorFamily>();
}

/// AC (tier b) / GTW-270 — the REAL `assets/content/armor/` folder resolves
/// into a stem-keyed [`ArmorRegistry`] through the Load code path.
#[test]
fn real_asset_resolves_armor_registry_keyed_by_filename() {
    suite::real_asset_resolves_registry::<ArmorFamily>();
}

/// GTW-269 (mirroring GTW-297 `AC3b`) — the REAL auto-battle path: the
/// `AutoBattlePlugin`'s `seed_load_fallbacks` runs on `Startup` AND a real
/// `AssetServer` is present (the GUI auto-battle launch). The empty
/// `ArmorRegistry::default()` seed must NOT shadow the real folder resolve: with an
/// `AssetServer` present the seed must NOT insert an empty registry, so the generic
/// content-family resolve (which only runs while the registry is ABSENT) populates
/// it from `assets/content/armor/*.armor.ron`.
///
/// This reproduces the bug's exact preconditions on the real code path: a
/// `GdtfLoadTestAppBuilder` app (live `AssetServer` rooted at the workspace `assets/`)
/// with the genuine `seed_load_fallbacks` system registered on `Startup`, exactly as
/// `AutoBattlePlugin::build` wires it. The assertions encode the fix.
///
/// PIN: if the seed reverts to inserting `ArmorRegistry::default()` UNCONDITIONALLY,
/// the registry is present from `Startup`, the absence-gated resolve SKIPS the folder,
/// the registry stays empty, `spec("flak_vest")` returns `None`, and this test goes
/// red. Family-bespoke: NEVER genericized away (GTW-580 P9).
#[test]
fn seeded_startup_does_not_shadow_real_armor_resolution() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    // Wire the REAL auto-battle Startup seed (the path AutoBattlePlugin registers),
    // minus the unrelated `drive_past_menu` that needs the RunningState machinery.
    app.add_systems(Startup, seed_load_fallbacks);

    // Signal-poll the ArmorRegistry insert (not a fixed frame count): the real resolve
    // only inserts the registry ONCE it is fully built from the folder (it stays ABSENT
    // while members are still resolving), so on the GOOD path existence implies the
    // authored armor is present. The cap is a safety net (GTW-305). The seed-shadow
    // regression instead inserts an EMPTY registry at Startup, which the assertions
    // below catch immediately.
    advance_until_resource_exists::<ArmorRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<ArmorRegistry>() {
        // Pin: with the AssetServer present the empty seed must NOT win — the real folder
        // resolve must populate a registry that holds the authored armor.
        assert!(
            !registry.is_empty(),
            "the real folder resolve must populate the registry, not leave the empty seed",
        );
        assert!(
            registry
                .spec(&ArmorName::new("flak_vest".to_owned()))
                .is_some(),
            "the registry must hold `flak_vest` — the empty seed must NOT have shadowed the resolve",
        );
        assert!(
            registry
                .spec(&ArmorName::new("carapace_plate".to_owned()))
                .is_some(),
            "the resolved registry must also hold `carapace_plate` (the empty seed held neither)",
        );
    }

    // The gate waited for the REAL registry: the machine releases past Load with it
    // present (Intro is TRANSIENT — probe via `load_released`, GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with the Startup seed present, Load must still release to Intro (or beyond) once the \
         real armor resolves; last AppState was {:?}",
        app_state(&app),
    );
}
