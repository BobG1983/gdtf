//! GTW-409 (gate remediation AC2): `AppState::Load` loads `assets/content/themes/` through
//! the `RonAsset<ThemeSpec>` loader, builds a theme-keyed [`ThemeCatalogRegistry`] from
//! the loaded files (keyed by each file's DECLARED [`LevelTheme`], not the filename stem),
//! and gates the Load→Intro transition on it.
//!
//! This file adds the MISSING real-asset Tier (b) test: the 19 pre-existing integration
//! tests all seed [`ThemeCatalogRegistry::default()`], masking any regression in
//! `resolve_themes`. This test does NOT seed the default — it drives the REAL
//! `resolve_themes` code path over the real `assets/content/themes/` folder and asserts
//! that the registry POPULATES with all three shipped [`LevelTheme`]s, each carrying a
//! catalog whose `default_floor` resolves to a [`Floor`](gdtf_battle_sim::level::CatalogTileKind::Floor)-kind
//! tile.
//!
//! **Additive only** — no production code is changed.
//!
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace `assets/`.
//!   Drives the REAL `AppState::Load` flow; `advance_until_resource_exists::<ThemeCatalogRegistry>`
//!   signal-polls until `resolve_themes` publishes the registry, then asserts POPULATED.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts presence / count / variant-routing / default-floor
//! resolves ONLY — no authored magnitudes (`HP` / armor / `move_cost` / atlas index) pinned.
//! Mirrors the terrain test's style (`real_asset_resolves_terrain_registry_keyed_by_filename`).
//!
//! ROBUST: asserts only against what IS authored today (the three shipped themes), so
//! future theme additions cannot redden it (it asserts `len() >= 3`, not `== 3`).

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::level::{CatalogTileKind, LevelTheme, ThemeCatalogRegistry};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_ui::theme::GdtfTheme;

/// Generous SAFETY-NET cap for the real-asset Tier (b) `advance_until` waits gated on an
/// async asset load resolving. The themes folder load shares the `AssetServer` with every
/// other loaded folder (theme / tuning / weapons / armor / terrain / injuries + the
/// presenter's tile-sheet loads), so under parallel `cargo` contention the async resolve
/// has NO fixed frame count. These waits key off the resolved SIGNAL; the cap is a safety
/// net against a genuine never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// AC (tier b) / GTW-409 AC2 — with a real `AssetServer` rooted at the workspace
/// `assets/`, entering `Load` loads `assets/content/themes/*.theme.ron` and builds a
/// [`ThemeCatalogRegistry`] keyed by each file's DECLARED [`LevelTheme`] (NOT the
/// filename stem — the `resolve_themes` design). Proves:
///
/// - The registry RESOLVES POPULATED: non-empty, containing all three shipped themes
///   ([`IndustrialHive`](LevelTheme::IndustrialHive) / [`Underhive`](LevelTheme::Underhive)
///   / [`SumpWaste`](LevelTheme::SumpWaste)) as declared keys.
/// - Each theme's catalog has a non-empty tile palette.
/// - Each theme's `default_floor()` resolves to a [`Floor`](CatalogTileKind::Floor)-kind
///   tile (not `None`, not a Wall/Cover/Scatter/Slab) — the C3 invariant.
/// - The Load gate WAITED for the registry: the machine reaches Intro with the registry
///   present, proving the gate clause fired on a real registry.
///
/// Does NOT seed [`ThemeCatalogRegistry::default()`] — seeding would mask the very
/// regression under test. Existence of the resource here proves `resolve_themes` published
/// it from the real folder (not a hand-seeded default).
///
/// VALUE-AGNOSTIC: asserts presence / count / variant-routing / default-floor resolves —
/// NEVER any authored magnitude (`move_cost` / `HP` / armor / atlas index). Mirrors the terrain
/// precedent (`real_asset_resolves_terrain_registry_keyed_by_filename`).
#[test]
fn real_asset_resolves_theme_catalog_registry() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async themes folder load: wait until resolve_themes inserts the
    // ThemeCatalogRegistry, not a fixed frame count. Cap is a safety net (GTW-305).
    // DELIBERATELY do NOT seed ThemeCatalogRegistry::default() — existence here proves
    // the REAL resolve_themes published it from the assets/content/themes/ folder.
    advance_until_resource_exists::<ThemeCatalogRegistry>(&mut app, LOAD_SAFETY_NET);

    // --- Non-empty + all three shipped LevelThemes present --------------------------------
    if let Some(registry) = app.world().get_resource::<ThemeCatalogRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved ThemeCatalogRegistry must carry the authored (non-empty) theme catalogs",
        );
        // The three shipped themes must all be present as declared keys. `>= 3` not `== 3`
        // so future additions cannot redden this test.
        assert!(
            registry.len() >= 3,
            "the registry must hold all three shipped themes; found {} catalog(s)",
            registry.len(),
        );
        assert!(
            registry.catalog(LevelTheme::IndustrialHive).is_some(),
            "the registry must hold the `IndustrialHive` catalog \
             (declared by industrial_hive.theme.ron)",
        );
        assert!(
            registry.catalog(LevelTheme::Underhive).is_some(),
            "the registry must hold the `Underhive` catalog \
             (declared by underhive.theme.ron)",
        );
        assert!(
            registry.catalog(LevelTheme::SumpWaste).is_some(),
            "the registry must hold the `SumpWaste` catalog \
             (declared by sump_waste.theme.ron)",
        );

        // --- Each catalog non-empty + default_floor resolves to a Floor-kind tile (C3) -----
        for theme in [
            LevelTheme::IndustrialHive,
            LevelTheme::Underhive,
            LevelTheme::SumpWaste,
        ] {
            let Some(catalog) = registry.catalog(theme) else {
                continue; // already asserted above; skip to keep the loop clean
            };
            assert!(
                !catalog.is_empty(),
                "the {theme:?} catalog must hold at least one tile",
            );

            // C3: default_floor() must resolve (the authored default_floor key exists in the
            // tile palette) and must be a Floor-kind tile. Value-agnostic — variant only,
            // never move_cost magnitude.
            let floor_tile = catalog.default_floor();
            assert!(
                floor_tile.is_some(),
                "the {theme:?} catalog's default_floor() must resolve to a tile (the authored \
                 default_floor key must exist in the catalog's tile palette — C3 invariant)",
            );
            if let Some(tile) = floor_tile {
                assert!(
                    matches!(tile.kind, CatalogTileKind::Floor { .. }),
                    "the {theme:?} catalog's default_floor tile must be a Floor-kind \
                     (got {:?}) — C3: the default floor is walkable ground, not a wall/cover/slab",
                    tile.kind,
                );
            }
        }
    }

    // --- Load gate WAITED for the registry -----------------------------------------------
    // The machine must reach Intro once every folder (incl. themes) resolves. The registry
    // is present when it does, proving the gate clause fired on a real registry (not a
    // hand-seeded default).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once all folders (incl. themes) \
         resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<ThemeCatalogRegistry>().is_some(),
        "a ThemeCatalogRegistry must be present when Load reaches Intro \
         (the gate clause waited for it — not a hand-seeded default)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme resolved alongside the theme catalog (the gated transition fired)",
    );
}
