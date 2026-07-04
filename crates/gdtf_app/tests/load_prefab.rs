//! GTW-489 (child T05c of the GTW-476 refactor) C1: with a REAL `AssetServer` rooted at a
//! TEST fixture folder, entering `AppState::Load` loads the UUID-keyed prefab
//! fragments from the `content/maps/<theme>/<size>/*.prefab.ron` root THROUGH the actual
//! `resolve_prefabs` branch (not a hand-inserted resource) and builds the
//! [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry), bucketed by each spec's
//! `(theme, size, role)` — keyed on the stable [`ThemeUuid`].
//!
//! GTW-494 (T08) RETIRED the legacy game-side `resolve_prefabs` from the app Load flow, so the
//! `resolve_prefabs` branch this test drives is now the ONLY live prefab resolver in the app.
//! The fragments live under `content/maps/`. This test therefore points
//! a real `AssetServer` at a TEST fixture root (`tests/fixtures/prefab_root/`) whose
//! `content/` tree materializes ONLY the overridden `maps/` subdir (the GTW-580 fixture
//! convention: a fixture root carries just what it overrides). Every other content family's
//! folder is absent, so each fail-closes to its EMPTY registry (the no-strand guarantee) —
//! the rest of the Load gate still clears, the resolve branch runs end-to-end, and a new
//! content family requires ZERO edits to this fixture root.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts the registry POPULATES and buckets under the expected
//! key + carries placements only — no authored magnitudes pinned. Mirrors `load_prefabs.rs` /
//! `load_terrain_model.rs` style.

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async load
/// resolving. The `content/maps/` folder load shares the `AssetServer` with every other loaded
/// folder, so under parallel `cargo` contention the async resolve has NO fixed frame count.
/// These waits key off the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The KNOWN theme UUID authored in `entry.prefab.ron`
/// (`Uuid::from_u128(0x0184_0a3e_0901)`).
const fn known_theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0901))
}

/// The absolute path of the prefab TEST fixture root (this crate's
/// `tests/fixtures/prefab_root/`), computed lexically from the manifest dir so it is
/// independent of the cwd.
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("prefab_root")
}

/// The 3x3x1 footprint the fixture prefab authors, or `None` (assert-fail) on a bad span.
fn fixture_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

/// GTW-489 C1 — with a real `AssetServer` rooted at the TEST fixture, entering `Load` loads
/// `content/maps/industrial_hive/3x3/entry.prefab.ron` through the ACTUAL
/// `resolve_prefabs` branch and builds the UUID-keyed [`PrefabRegistry`]. Proves:
///
/// - The [`PrefabRegistry`] RESOLVES POPULATED (non-empty).
/// - The fragment BUCKETS under the expected [`PrefabKey`]: its authored [`ThemeUuid`] +
///   the 3x3x1 footprint + the role-default [`SpawnRole::Fill`] (the file OMITS `role`).
/// - `prefabs_for` returns `>= 1` [`Prefab`](gdtf_battle_sim::level::Prefab) carrying
///   placements (the spec's one authored placement).
/// - The Load gate WAITED for the new registry: the machine reaches `Intro` with it present,
///   proving the GTW-489 gate clause fired on a real registry (not a hand-seeded default).
///
/// DELIBERATELY does NOT seed [`PrefabRegistry`] — existence here proves the REAL resolve
/// branch published it from the fixture folder. VALUE-AGNOSTIC: presence + key-bucketing +
/// placements only, no authored magnitudes.
#[test]
fn real_asset_resolves_prefab_registry_by_theme_uuid() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(fixture_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async `content/maps/` folder load until the new registry is inserted by
    // the real resolve branch. Cap is a safety net (GTW-305). DELIBERATELY do NOT seed
    // PrefabRegistry::default() — existence here proves the REAL resolve_prefabs branch
    // published it from the fixture folder.
    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    // --- PrefabRegistry: non-empty + buckets under the expected (theme, size, Fill) key ---
    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the authored fixture prefab — an empty \
             registry means resolve_prefabs is broken (fell back to the empty default)",
        );

        let Some(size) = fixture_size() else { return };
        // The file OMITS `role`, so it deserializes as the SpawnRole::Fill serde-default.
        let key = PrefabKey::new(known_theme_uuid(), size, SpawnRole::Fill);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (authored ThemeUuid, 3x3x1, Fill) bucket must hold the fixture prefab — \
             bucketing by (theme, size, role) keyed on the ThemeUuid is the C1 contract",
        );

        // prefabs_for returns >= 1 Prefab carrying placements (the one authored placement).
        let carries_placements = bucket
            .iter()
            .any(|prefab| !prefab.spec().placements.is_empty());
        assert!(
            carries_placements,
            "the resolved prefab must carry its authored placement(s)",
        );
    }

    // --- Load gate WAITED for the new registry --------------------------------------------
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. the \
         prefabs) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<PrefabRegistry>().is_some(),
        "a PrefabRegistry must be present when Load reaches Intro (the GTW-489 gate clause \
         waited for it — not a hand-seeded default)",
    );
}
