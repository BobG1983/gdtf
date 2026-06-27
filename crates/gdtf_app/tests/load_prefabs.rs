//! GTW-418 (T1): `AppState::Load` loads `assets/content/maps/**/*.prefab.ron` through the
//! `RonAsset<PrefabSpec>` loader, builds a per-`(theme, size, spawn-role)`
//! [`PrefabRegistry`] from the loaded fragments (validated for >= 1 edge opening), and
//! gates the Load→Intro transition on it.
//!
//! This is the MANDATORY real-asset Tier (b) test (the GTW-409/417 lesson): it does NOT
//! seed [`PrefabRegistry::default()`] — it drives the REAL `resolve_prefabs` code path over
//! the real `assets/content/maps/` folder and asserts the registry POPULATES (non-empty),
//! that enumeration by `(theme, size, spawn-role)` returns the authored sample prefab, and
//! that the authored prefab's name + reserved field resolve. It MUST FAIL if the loader is
//! broken (the empty-default fallback => empty registry => the assert reddens).
//!
//! **Additive only** — no production code is changed.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts presence / count / enumeration-routing / the authored
//! name + reserved-field presence ONLY — no authored terrain magnitudes (HP / move cost)
//! pinned. Mirrors the themes test's style (`real_asset_resolves_theme_catalog_registry`).
//!
//! ROBUST: asserts only against what IS authored today (the one shipped sample prefab), so
//! future prefab additions cannot redden it (it asserts `len() >= 1`, not `== 1`, and that
//! the sample's bucket is non-empty).

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, PrefabKey, PrefabRegistry, SpawnRole,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset Tier (b) `advance_until` waits gated on an
/// async asset load resolving. The maps folder load shares the `AssetServer` with every
/// other loaded folder, so under parallel `cargo` contention the async resolve has NO
/// fixed frame count. These waits key off the resolved SIGNAL; the cap is a safety net
/// against a genuine never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// GTW-418 T1 / AC2+AC3+AC4 — with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads `assets/content/maps/**/*.prefab.ron` and builds a
/// [`PrefabRegistry`] bucketed by each fragment's `(theme, size, spawn-role)`. Proves:
///
/// - The registry RESOLVES POPULATED: non-empty (`len() >= 1`).
/// - Enumeration by `(theme, size, spawn-role)` returns the authored sample prefab: the
///   `(IndustrialHive, 3x3x1, Fill)` bucket holds the `entry_room` fragment.
/// - The authored sample resolves its name + the C6 invariant (>= 1 edge opening) + the
///   reserved `ai_route_nodes` field carries the authored waypoint.
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
    // do NOT seed PrefabRegistry::default() — existence here proves the REAL resolve_prefabs
    // published it from the assets/content/maps/ folder.
    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        // --- Non-empty: the loader populated it from the real maps folder -----------------
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the authored (non-empty) prefabs — an \
             empty registry means resolve_prefabs is broken (fell back to the empty default)",
        );
        assert!(
            !registry.is_empty(),
            "the registry must hold at least the one shipped sample prefab; found {}",
            registry.len(),
        );

        // --- Enumeration by (theme, size, spawn-role) returns the sample prefab (C3) ------
        let size = GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1));
        assert!(
            size.is_ok(),
            "the 3x3x1 sample footprint must be a valid GridSize"
        );
        let Some(size) = size.ok() else { return };
        let key = PrefabKey::new(LevelTheme::IndustrialHive, size, SpawnRole::Fill);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (IndustrialHive, 3x3x1, Fill) bucket must hold the authored entry_room \
             sample prefab — enumeration by (theme, size, spawn-role) is the C3 contract",
        );

        // --- The authored sample resolves its name + C6 + reserved field ------------------
        let entry = bucket.iter().find(|p| &***p.name() == "entry_room");
        assert!(
            entry.is_some(),
            "the sample prefab keyed by its file stem `entry_room` must be present in its \
             (theme, size, role) bucket",
        );
        if let Some(prefab) = entry {
            // C6: the loader only admits prefabs with >= 1 edge opening, so the registered
            // sample MUST carry one.
            assert!(
                prefab.spec().edge_opening_count() != 0,
                "a registered prefab must author >= 1 edge opening (C6 — the loader rejects \
                 zero-opening prefabs fail-closed, so any registered one has a seam)",
            );
            // C1 / C5: the reserved ai_route_nodes field carries the authored waypoint
            // (value-agnostic — its mere presence proves the reserved field round-tripped).
            assert!(
                !prefab.spec().ai_route_nodes.is_empty(),
                "the sample prefab's reserved ai_route_nodes must carry its authored waypoint",
            );
        }
    }

    // --- Load gate WAITED for the registry -----------------------------------------------
    // The machine must reach Intro once every folder (incl. maps) resolves. The registry is
    // present when it does, proving the gate clause fired on a real registry (not a
    // hand-seeded default).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once all folders (incl. maps) \
         resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<PrefabRegistry>().is_some(),
        "a PrefabRegistry must be present when Load reaches Intro (the gate clause waited \
         for it — not a hand-seeded default)",
    );
}
