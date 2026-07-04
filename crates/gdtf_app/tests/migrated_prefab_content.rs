//! GTW-490 C3 / GTW-492: with a REAL `AssetServer` rooted at the SHIPPED workspace `assets/`,
//! entering `AppState::Load` loads the MIGRATED prefab fragments from the NEW
//! `assets/content/maps/<theme>/<size>/*.prefab.ron` root THROUGH the actual GTW-489
//! `resolve_prefabs` loader (not a hand-inserted resource) and builds the
//! [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry), bucketed by each spec's
//! `(theme, size, role)` keyed on the stable [`ThemeUuid`].
//!
//! Unlike the GTW-489 C1 test (which pointed at a TEST fixture because the shipped `content/maps/`
//! root was empty), THIS migration ticket fills the real `assets/content/maps/` tree, so the test
//! loads the SHIPPED prefabs directly via [`GdtfLoadTestAppBuilder::new`].
//!
//! C3: the [`PrefabRegistry`] is non-empty and every migrated prefab yields `>= 1`
//! placement.
//!
//! GTW-492 (T07b): the two 12x12 deployment prefabs now carry the authored deployment roles
//! ([`SpawnRole::Player`] / [`SpawnRole::Enemy`]) the procgen assembler requires — the
//! migration had deferred the role to "the assembler", which now consumes these fragments.
//! The 3x3 `entry_room` still OMITS `role` (serde-default [`SpawnRole::Fill`]). This test pins
//! that shipped role distribution.
//!
//! VALUE-AGNOSTIC (gate 4a / C6): presence + bucketing + placement-count + role only — no
//! authored magnitude pinned.

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset waits gated on an async load resolving (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] every shipped prefab draws from (matches the
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

/// GTW-490 C3 / GTW-492 — the SHIPPED migrated prefabs resolve through the real GTW-489
/// loader into `PrefabRegistry`, each carrying placements and bucketed under its authored
/// `(theme, size, role)` key: the 3x3 `entry_room` under role-default Fill, the two 12x12
/// deployment prefabs under the GTW-492-authored Player / Enemy roles.
#[test]
fn shipped_migrated_prefabs_resolve_with_placements_and_role_default() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the migrated shipped prefabs — an empty \
             registry means resolve_prefabs fell back to the empty default (C3)",
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
            let key = PrefabKey::new(theme, size, role);
            let bucket = registry.prefabs_for(&key);
            assert!(
                !bucket.is_empty(),
                "the migrated {label} bucket (IndustrialHive ThemeUuid, role {role:?}) must hold \
                 its prefab(s) — shipped role distribution (C3 / GTW-492)",
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

    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once every folder \
         (incl. the migrated prefabs) resolves; last AppState was {:?}",
        app_state(&app),
    );
}
