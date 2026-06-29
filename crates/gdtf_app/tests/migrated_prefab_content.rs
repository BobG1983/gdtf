//! GTW-490 C3: with a REAL `AssetServer` rooted at the SHIPPED workspace `assets/`, entering
//! `AppState::Load` loads the MIGRATED v2 prefab fragments from the NEW
//! `assets/maps/<theme>/<size>/*.prefab_v2.ron` root THROUGH the actual GTW-489
//! `resolve_prefabs_v2` loader (not a hand-inserted resource) and builds the
//! [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2), bucketed by each spec's
//! `(theme, size, role)` keyed on the stable [`ThemeUuid`].
//!
//! Unlike the GTW-489 C1 test (which pointed at a TEST fixture because the shipped `maps/`
//! root was empty), THIS migration ticket fills the real `assets/maps/` tree, so the test
//! loads the SHIPPED v2 prefabs directly via [`GdtfLoadTestAppBuilder::new`].
//!
//! C3: the [`PrefabRegistry2`] is non-empty, every migrated prefab yields `>= 1` placement,
//! and the role-default `SpawnRole::Fill` is applied (all migrated prefabs OMIT `role`).
//!
//! VALUE-AGNOSTIC (gate 4a / C6): presence + bucketing + placement-count + role-default only —
//! no authored magnitude pinned.

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey2, PrefabRegistry2, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset waits gated on an async load resolving (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] every shipped v2 prefab draws from (matches the
/// migrated `industrial_hive.terrain_theme.ron` key).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// Build a `(w, h, 1)` footprint [`GridSize`], or `None` (assert-fail) on a bad span.
fn footprint(width: u8, height: u8) -> Option<GridSize> {
    GridSize::new(
        GridWidth::new(width),
        GridHeight::new(height),
        GridLevels::new(1),
    )
    .ok()
}

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// GTW-490 C3 — the SHIPPED migrated v2 prefabs resolve through the real GTW-489 loader into
/// `PrefabRegistry2`, each carrying placements and bucketed under the role-default Fill key.
#[test]
fn shipped_migrated_prefabs_resolve_with_placements_and_role_default() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry2>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry2>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry2 must carry the migrated shipped v2 prefabs — an empty \
             registry means resolve_prefabs_v2 fell back to the empty default (C3)",
        );

        // The migrated prefabs OMIT `role`, so each buckets under the role-default Fill (C3).
        // The 3x3 entry_room and the two 12x12 deployment prefabs are the two footprints.
        let Some(size_3) = footprint(3, 3) else {
            return;
        };
        let Some(size_12) = footprint(12, 12) else {
            return;
        };
        let theme = industrial_hive_theme();

        for (size, label) in [(size_3, "3x3"), (size_12, "12x12")] {
            let key = PrefabKey2::new(theme, size, SpawnRole::Fill);
            let bucket = registry.prefabs_for(&key);
            assert!(
                !bucket.is_empty(),
                "the migrated {label} bucket (IndustrialHive ThemeUuid, role-default Fill) must \
                 hold its v2 prefab(s) — role defaults applied (C3)",
            );
            // Every migrated prefab in the bucket carries >= 1 placement (the old walls/scatter).
            for prefab in bucket {
                assert!(
                    !prefab.spec().placements.is_empty(),
                    "the migrated {label} prefab must carry >= 1 placement (the old walls/scatter \
                     refs mapped to migrated TerrainUuids) (C3)",
                );
                // Role-default Fill is applied on every migrated prefab (the file OMITS `role`).
                assert_eq!(
                    prefab.spec().role,
                    SpawnRole::Fill,
                    "the migrated {label} prefab must apply the serde-default role Fill (C3)",
                );
            }
        }
    }

    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. the migrated v2 \
         prefabs) resolves; last AppState was {:?}",
        app_state(&app),
    );
}
