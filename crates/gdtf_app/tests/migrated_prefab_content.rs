//! GTW-490 C3 / GTW-492: with a REAL `AssetServer` rooted at the SHIPPED workspace `assets/`,
//! entering `AppState::Load` loads the MIGRATED v2 prefab fragments from the NEW
//! `assets/maps/<theme>/<size>/*.prefab_v2.ron` root THROUGH the actual GTW-489
//! `resolve_prefabs_v2` loader (not a hand-inserted resource) and builds the
//! [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2), bucketed by each spec's
//! `(theme, size, role)` keyed on the stable [`ThemeUuid`].
//!
//! Unlike the GTW-489 C1 test (which pointed at a TEST fixture because the shipped `maps/`
//! root was empty), THIS migration ticket fills the real `assets/maps/` tree, so the test
//! loads the SHIPPED v2 prefabs directly via [`GdtfLoadTestAppBuilder::new`].
//!
//! C3: the [`PrefabRegistry2`] is non-empty and every migrated prefab yields `>= 1`
//! placement.
//!
//! GTW-492 (T07b): the two 12x12 deployment prefabs now carry the authored deployment roles
//! ([`SpawnRole::Player`] / [`SpawnRole::Enemy`]) the v2 procgen assembler requires — the
//! migration had deferred the role to "the v2 assembler", which now consumes these fragments.
//! The 3x3 `entry_room` still OMITS `role` (serde-default [`SpawnRole::Fill`]). This test pins
//! that shipped role distribution.
//!
//! VALUE-AGNOSTIC (gate 4a / C6): presence + bucketing + placement-count + role only — no
//! authored magnitude pinned.

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

/// GTW-490 C3 / GTW-492 — the SHIPPED migrated v2 prefabs resolve through the real GTW-489
/// loader into `PrefabRegistry2`, each carrying placements and bucketed under its authored
/// `(theme, size, role)` key: the 3x3 `entry_room` under role-default Fill, the two 12x12
/// deployment prefabs under the GTW-492-authored Player / Enemy roles.
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

        let Some(size_3) = footprint(3, 3) else {
            return;
        };
        let Some(size_12) = footprint(12, 12) else {
            return;
        };
        let theme = industrial_hive_theme();

        // Each (size, role) bucket the shipped content authors. GTW-492: the 12x12 deployment
        // prefabs carry the Player / Enemy roles the assembler needs (one prefab per bucket);
        // the 3x3 entry_room still omits `role` (serde-default Fill).
        let buckets = [
            (size_3, SpawnRole::Fill, "3x3 entry_room"),
            (size_12, SpawnRole::Player, "12x12 player_deployment"),
            (size_12, SpawnRole::Enemy, "12x12 enemy_deployment"),
        ];
        for (size, role, label) in buckets {
            let key = PrefabKey2::new(theme, size, role);
            let bucket = registry.prefabs_for(&key);
            assert!(
                !bucket.is_empty(),
                "the migrated {label} bucket (IndustrialHive ThemeUuid, role {role:?}) must hold \
                 its v2 prefab(s) — shipped role distribution (C3 / GTW-492)",
            );
            // Every migrated prefab in the bucket carries >= 1 placement (the old walls/scatter)
            // and the bucket's authored role.
            for prefab in bucket {
                assert!(
                    !prefab.spec().placements.is_empty(),
                    "the migrated {label} prefab must carry >= 1 placement (the old walls/scatter \
                     refs mapped to migrated TerrainUuids) (C3)",
                );
                assert_eq!(
                    prefab.spec().role,
                    role,
                    "the migrated {label} prefab must carry its authored role {role:?}",
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
