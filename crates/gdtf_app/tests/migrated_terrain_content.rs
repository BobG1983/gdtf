//! GTW-490 C2 / C4: with a REAL `AssetServer` rooted at the SHIPPED workspace `assets/`,
//! entering `AppState::Load` loads the MIGRATED per-theme terrain + theme content from
//! `assets/content/terrain/<theme>/*.terrain_def.ron` + `*.terrain_theme.ron` THROUGH the actual
//! GTW-487 `resolve_terrain_defs` / `resolve_theme_defs` loader (not a hand-inserted
//! resource) and builds the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry), keyed by each def's OWN
//! UUID.
//!
//! Unlike the GTW-487 C1 test (which pointed at a TEST fixture because the shipped content
//! was empty), THIS migration ticket fills the real `assets/content/terrain/` tree, so the test loads
//! the SHIPPED dirs directly via [`GdtfLoadTestAppBuilder::new`] (workspace `assets/` root).
//!
//! - **C2**: the [`TerrainDefRegistry`] is not-empty with the EXPECTED COUNT (all migrated
//!   pieces present — the reconciled flat 8 for `industrial_hive` plus the 4+4 for the other
//!   two themes — and EACH of the 8 flat-piece UUIDs resolves, proving none was silently
//!   dropped).
//!   The [`UuidThemeRegistry`] holds all 3 migrated themes resolvable by [`ThemeUuid`], each
//!   resolving its `default_floor` [`TerrainUuid`].
//! - **C4**: every migrated terrain def parsed into a [`TerrainDef`] and every theme into a
//!   [`UuidThemeDef`] via the real loader (presence proves parse-OK); a sim-side tag present on
//!   a wall round-trips, and is on the SIM side of the def (`tags`), NEVER on `presenter_kind`.
//!
//! VALUE-AGNOSTIC (gate 4a / C6): NO authored magnitude is pinned — presence + count +
//! known-UUID resolution + cross-reference consistency only.

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainTag, TerrainUuid},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async load
/// resolving — keyed off the resolved SIGNAL, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The total number of migrated terrain defs across all three themes: the reconciled flat 8
/// (`industrial_hive`) + 4 (`underhive`) + 4 (`sump_waste`) = 16, PLUS the GTW-469 EW-wall
/// companions (2 `industrial_hive` + 1 `underhive` + 1 `sump_waste` = 4) = 20, PLUS the GTW-470
/// orientation/direction door + stair tiles in `industrial_hive` (2 doors + 4 stairs = 6) = 26,
/// PLUS the GTW-543 `heavy_bolter_emplacement` in `industrial_hive` (1) = 27. A COUNT, not a
/// magnitude — it proves no terrain def was dropped (C2), not any balance value.
const MIGRATED_TERRAIN_DEF_COUNT: usize = 27;

/// The number of migrated themes (`industrial_hive`, `underhive`, `sump_waste`).
const MIGRATED_THEME_COUNT: usize = 3;

/// Wrap a `0x0184_0a..` u128 into a [`TerrainUuid`] — the authored-constant scheme used across
/// the migrated terrain defs / theme files / prefab placements.
const fn terrain_uuid(low: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(low))
}

/// Wrap a `0x0184_0a..` u128 into a [`ThemeUuid`].
const fn theme_uuid(low: u128) -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(low))
}

/// The 8 reconciled `industrial_hive` flat-piece [`TerrainUuid`]s, in authored order
/// (`barricade`, `bulkhead_wall`, `debris_pile`, `deck_floor`, `deck_slab`, `gantry_slab`,
/// `heavy_bulkhead`, `supply_crate`). Each MUST resolve in the registry — the C2 "none silently
/// dropped" proof.
const fn industrial_hive_flat_eight() -> [TerrainUuid; 8] {
    [
        terrain_uuid(0x0184_0a91_0001),
        terrain_uuid(0x0184_0a91_0002),
        terrain_uuid(0x0184_0a91_0003),
        terrain_uuid(0x0184_0a91_0004),
        terrain_uuid(0x0184_0a91_0005),
        terrain_uuid(0x0184_0a91_0006),
        terrain_uuid(0x0184_0a91_0007),
        terrain_uuid(0x0184_0a91_0008),
    ]
}

/// The 3 migrated [`ThemeUuid`]s paired with the [`TerrainUuid`] each authors as its
/// `default_floor` (`industrial_hive` -> `deck_floor`, `underhive` -> `rockcrete_floor`,
/// `sump_waste` -> `sludge_floor`). C2 asserts each theme resolves AND its `default_floor`
/// resolves to the expected terrain UUID.
const fn migrated_themes() -> [(ThemeUuid, TerrainUuid); 3] {
    [
        (theme_uuid(0x0184_0a90_0001), terrain_uuid(0x0184_0a91_0004)),
        (theme_uuid(0x0184_0a90_0002), terrain_uuid(0x0184_0a92_0001)),
        (theme_uuid(0x0184_0a90_0003), terrain_uuid(0x0184_0a93_0001)),
    ]
}

/// The migrated `industrial_hive` `bulkhead_wall` UUID — a `Wall` carrying the sim-side
/// `[BlocksVision, BlocksPathfinding]` tags (C4 tag round-trip).
const fn bulkhead_wall_uuid() -> TerrainUuid {
    terrain_uuid(0x0184_0a91_0002)
}

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// GTW-490 C2 / C4 — the SHIPPED migrated terrain + theme content resolves through the real
/// GTW-487 loader into the UUID-keyed registries.
#[test]
fn shipped_migrated_terrain_and_theme_content_resolves_by_uuid() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<TerrainDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    // --- C2: TerrainDefRegistry — non-empty, expected count, every flat-8 UUID present ------
    if let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainDefRegistry must carry the migrated shipped terrain defs",
        );
        assert_eq!(
            registry.len(),
            MIGRATED_TERRAIN_DEF_COUNT,
            "the registry must hold every migrated terrain def (reconciled flat 8 + 4 + 4, plus \
             the GTW-469 EW-wall companions 2 + 1 + 1) — a count mismatch means a piece was \
             silently dropped (C2)",
        );
        for uuid in industrial_hive_flat_eight() {
            assert!(
                registry.def(&uuid).is_some(),
                "the reconciled industrial_hive flat-8 piece {uuid:?} must resolve — none of the \
                 source-of-truth flat 8 may be dropped (C2)",
            );
        }

        // --- C4: a sim-side tag round-trips on the SIM side, never on presenter_kind ---------
        if let Some(wall) = registry.def(&bulkhead_wall_uuid()) {
            assert!(
                matches!(wall.sim_kind, TerrainSimKind::Wall { .. }),
                "the migrated bulkhead_wall must be a Wall sim_kind",
            );
            assert!(
                wall.tags.contains(&TerrainTag::BlocksVision)
                    && wall.tags.contains(&TerrainTag::BlocksPathfinding),
                "the authored sim-side wall tags must round-trip through the loader (C4)",
            );
            // `presenter_kind` is a closed enum with NO tags field — the compiler enforces tags
            // live on the SIM side of the def (`tags`), never on the presenter half (C4).
        }
    }

    // --- C2: UuidThemeRegistry — all 3 themes resolvable, each default_floor resolving -------
    if let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved UuidThemeRegistry must carry the migrated shipped theme defs",
        );
        assert_eq!(
            registry.len(),
            MIGRATED_THEME_COUNT,
            "the registry must hold all 3 migrated themes (industrial_hive, underhive, \
             sump_waste)",
        );
        for (theme, expected_floor) in migrated_themes() {
            assert!(
                registry.def(&theme).is_some(),
                "the migrated theme {theme:?} must resolve by its ThemeUuid (C2)",
            );
            assert_eq!(
                registry.default_floor(&theme),
                Some(expected_floor),
                "the migrated theme {theme:?} must resolve its authored default_floor TerrainUuid \
                 (C2 cross-reference consistency)",
            );
            // The theme's default_floor must itself resolve to a terrain def (the migrated floor
            // piece) — a dangling reference would mean a dropped or mis-keyed floor.
            if let Some(terrain) = app.world().get_resource::<TerrainDefRegistry>() {
                assert!(
                    terrain.def(&expected_floor).is_some(),
                    "the theme {theme:?} default_floor TerrainUuid must resolve to a migrated \
                     terrain def (no dangling reference)",
                );
            }
        }
    }

    // --- Load gate WAITED for both new registries (real resolve, not seeded) -----------------
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. the migrated \
         per-theme terrain/) resolves; last AppState was {:?}",
        app_state(&app),
    );
}
