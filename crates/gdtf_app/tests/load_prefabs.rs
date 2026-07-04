//! GTW-494 (child T08 of the GTW-476 refactor): `AppState::Load` loads the prefab
//! fragments from `assets/content/maps/<theme>/<size>/*.prefab.ron` through the GTW-489
//! `resolve_prefabs` loader, builds the UUID-keyed [`PrefabRegistry`] from them, and
//! gates the Load→Intro transition on it.
//!
//! This file was MIGRATED off the retired flat-dir `resolve_prefabs` per-file prefab model
//! (GTW-418) onto the
//! UUID model: GTW-494 removed the old game-side loader, so the `PrefabRegistry` is now
//! the ONLY prefab resolver in the Load flow (the procgen pipeline consumes it, GTW-492). It
//! does NOT seed the registry — it drives the REAL Load branch over the shipped
//! `assets/content/maps/` content and asserts the registry POPULATES (not-empty) and a KNOWN
//! `(theme, size, role)` bucket resolves (the C2 contract: not-empty + a known key resolves,
//! via the real Load branch, not a seeded default).
//!
//! VALUE-AGNOSTIC (gate 4a): asserts presence / bucketing / placement-count ONLY — no
//! authored terrain magnitudes pinned.

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset Tier (b) `advance_until` waits gated on an
/// async asset load resolving. The maps folder load shares the `AssetServer` with every
/// other loaded folder, so under parallel `cargo` contention the async resolve has NO
/// fixed frame count. These waits key off the resolved SIGNAL; the cap is a safety net
/// against a genuine never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] every shipped prefab draws from (matches
/// the migrated `industrial_hive.terrain_theme.ron` key).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// GTW-494 C2 — with a real `AssetServer` rooted at the workspace `assets/`, entering `Load`
/// loads `assets/content/maps/<theme>/<size>/*.prefab.ron` and builds the UUID-keyed
/// [`PrefabRegistry`] bucketed by each fragment's `(theme, size, role)` through the ACTUAL
/// `resolve_prefabs` branch. Proves:
///
/// - The registry RESOLVES POPULATED (non-empty).
/// - A KNOWN `(IndustrialHive, 12x12, Player)` bucket resolves the migrated player-deployment
///   fragment (enumeration by `(theme, size, role)` keyed on the stable `ThemeUuid`).
/// - The Load gate WAITED for the registry: the machine reaches Intro with the registry
///   present, proving the gate clause fired on a real registry.
///
/// Does NOT seed [`PrefabRegistry::default()`] — seeding would mask the very regression
/// under test. Existence of the resource here proves `resolve_prefabs` published it from
/// the real folder (not a hand-seeded default).
#[test]
fn real_asset_resolves_prefab_registry() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async maps folder load: wait until resolve_prefabs inserts the
    // PrefabRegistry, not a fixed frame count. Cap is a safety net (GTW-305). DELIBERATELY
    // do NOT seed PrefabRegistry::default() — existence here proves the REAL
    // resolve_prefabs published it from the assets/content/maps/ folder.
    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the migrated (non-empty) prefabs — an \
             empty registry means resolve_prefabs is broken (fell back to the empty default)",
        );

        // --- Enumeration by (theme, size, role) returns the migrated player deployment -----
        let size = GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1));
        assert!(
            size.is_ok(),
            "the 12x12x1 deployment footprint must be a valid GridSize"
        );
        let Some(size) = size.ok() else { return };
        let key = PrefabKey::new(industrial_hive_theme(), size, SpawnRole::Player);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (IndustrialHive, 12x12, Player) bucket must hold the migrated player-deployment \
             prefab — enumeration by (theme, size, role) is the C2 contract",
        );
        // Every prefab in the bucket carries >= 1 placement (the migrated walls/scatter refs).
        for prefab in bucket {
            assert!(
                !prefab.spec().placements.is_empty(),
                "the migrated player-deployment prefab must carry >= 1 placement",
            );
        }
    }

    // --- Load gate WAITED for the registry -----------------------------------------------
    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once all folders \
         (incl. the maps) resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<PrefabRegistry>().is_some(),
        "a PrefabRegistry must be present when Load reaches Intro (the gate clause waited \
         for it — not a hand-seeded default)",
    );
}
