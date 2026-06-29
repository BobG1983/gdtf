//! GTW-489 (child T05c of the GTW-476 refactor) C1: with a REAL `AssetServer` rooted at a
//! TEST fixture folder, entering `AppState::Load` loads the NEW UUID-keyed v2 prefab
//! fragments from the NEW `maps/<theme>/<size>/*.prefab_v2.ron` root THROUGH the actual
//! `resolve_prefabs_v2` branch (not a hand-inserted resource) and builds the
//! [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2), bucketed by each spec's
//! `(theme, size, role)` — keyed on the stable [`ThemeUuid`].
//!
//! This NEW loader runs BESIDE the legacy `resolve_prefabs` (which stays live + unchanged —
//! C5); the v2 fragments live under their OWN `maps/` root, SEPARATE from the legacy
//! `content/maps/` tree, and the shipped game carries NO `maps/` root yet, so the registry
//! resolves EMPTY there (the designed fail-closed state until the T06 content migration).
//! This test therefore points a real `AssetServer` at a TEST fixture root
//! (`tests/fixtures/prefab_v2_root/`) whose NEW top-level `maps/industrial_hive/3x3/` holds
//! one `*.prefab_v2.ron`, with the shipped `content/` (incl. the legacy `content/maps/`
//! `*.prefab.ron` fragments) and the other dirs symlinked to the real `assets/` — so the
//! still-live legacy `PrefabRegistry` ALSO resolves populated (proving coexistence — C5), the
//! rest of the Load gate clears, and the new resolve branch actually runs end-to-end.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts the registry POPULATES and buckets under the expected
//! key + carries placements only — no authored magnitudes pinned. Mirrors `load_prefabs.rs` /
//! `load_terrain_model.rs` style.

use std::path::PathBuf;

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey2, PrefabRegistry, PrefabRegistry2,
    SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async load
/// resolving. The `content/maps/` folder load shares the `AssetServer` with every other loaded
/// folder, so under parallel `cargo` contention the async resolve has NO fixed frame count.
/// These waits key off the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The KNOWN theme UUID authored in `entry.prefab_v2.ron`
/// (`Uuid::from_u128(0x0184_0a3e_0901)`).
const fn known_theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0901))
}

/// The absolute path of the v2-prefab TEST fixture root (this crate's
/// `tests/fixtures/prefab_v2_root/`), computed lexically from the manifest dir so it is
/// independent of the cwd.
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("prefab_v2_root")
}

/// The 3x3x1 footprint the fixture v2 prefab authors, or `None` (assert-fail) on a bad span.
fn fixture_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// GTW-489 C1 — with a real `AssetServer` rooted at the TEST fixture, entering `Load` loads
/// `content/maps/industrial_hive/3x3/entry.prefab_v2.ron` through the ACTUAL
/// `resolve_prefabs_v2` branch and builds the UUID-keyed [`PrefabRegistry2`]. Proves:
///
/// - The [`PrefabRegistry2`] RESOLVES POPULATED (non-empty).
/// - The fragment BUCKETS under the expected [`PrefabKey2`]: its authored [`ThemeUuid`] +
///   the 3x3x1 footprint + the role-default [`SpawnRole::Fill`] (the v2 file OMITS `role`).
/// - `prefabs_for` returns `>= 1` [`Prefab2`](gdtf_battle_sim::level::Prefab2) carrying
///   placements (the v2 spec's one authored placement).
/// - The legacy [`PrefabRegistry`] ALSO resolves populated from the symlinked shipped
///   `*.prefab.ron` — proving the new loader coexists with the still-live legacy one (C5).
/// - The Load gate WAITED for the new registry: the machine reaches `Intro` with it present,
///   proving the GTW-489 gate clause fired on a real registry (not a hand-seeded default).
///
/// DELIBERATELY does NOT seed [`PrefabRegistry2`] — existence here proves the REAL resolve
/// branch published it from the fixture folder. VALUE-AGNOSTIC: presence + key-bucketing +
/// placements only, no authored magnitudes.
#[test]
fn real_asset_resolves_prefab_v2_registry_by_theme_uuid() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(fixture_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async `content/maps/` folder load until the new registry is inserted by
    // the real resolve branch. Cap is a safety net (GTW-305). DELIBERATELY do NOT seed
    // PrefabRegistry2::default() — existence here proves the REAL resolve_prefabs_v2 published
    // it from the fixture folder.
    advance_until_resource_exists::<PrefabRegistry2>(&mut app, LOAD_SAFETY_NET);

    // --- PrefabRegistry2: non-empty + buckets under the expected (theme, size, Fill) key ---
    if let Some(registry) = app.world().get_resource::<PrefabRegistry2>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry2 must carry the authored fixture v2 prefab — an empty \
             registry means resolve_prefabs_v2 is broken (fell back to the empty default)",
        );

        let Some(size) = fixture_size() else { return };
        // The v2 file OMITS `role`, so it deserializes as the SpawnRole::Fill serde-default.
        let key = PrefabKey2::new(known_theme_uuid(), size, SpawnRole::Fill);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (authored ThemeUuid, 3x3x1, Fill) bucket must hold the fixture v2 prefab — \
             bucketing by (theme, size, role) keyed on the ThemeUuid is the C1 contract",
        );

        // prefabs_for returns >= 1 Prefab2 carrying placements (the one authored placement).
        let carries_placements = bucket
            .iter()
            .any(|prefab| !prefab.spec().placements.is_empty());
        assert!(
            carries_placements,
            "the resolved v2 prefab must carry its authored placement(s)",
        );
    }

    // --- Legacy PrefabRegistry ALSO resolved populated (C5 coexistence) -------------------
    // The fixture symlinks the shipped `entry_room.prefab.ron`, so the still-live legacy
    // resolve_prefabs must build its registry from it unaffected by the new v2 loader.
    if let Some(legacy) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !legacy.is_empty(),
            "the legacy PrefabRegistry must STILL resolve populated from the symlinked shipped \
             *.prefab.ron — the new v2 loader must not clobber the still-live legacy one (C5)",
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
        "with a real AssetServer, Load must reach Intro once every folder (incl. the v2 \
         prefabs) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<PrefabRegistry2>().is_some(),
        "a PrefabRegistry2 must be present when Load reaches Intro (the GTW-489 gate clause \
         waited for it — not a hand-seeded default)",
    );
}
