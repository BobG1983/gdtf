//! GTW-487 (child T05a of the GTW-476 refactor) C1: with a REAL `AssetServer` rooted at a
//! per-theme TEST fixture folder, entering `AppState::Load` loads the NEW UUID-keyed
//! terrain + theme models from `terrain/<theme>/*.terrain_def.ron` +
//! `*.terrain_theme.ron` THROUGH the actual `resolve_terrain_defs` / `resolve_theme_defs`
//! branch (not a hand-inserted resource) and builds the
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry), keyed by each def's OWN
//! UUID.
//!
//! These NEW loaders run BESIDE the legacy `resolve_terrain` / `resolve_themes` (which stay
//! live + unchanged — C5); the shipped game `assets/` carries NO per-theme layout yet, so
//! the registries resolve EMPTY there (the designed fail-closed state until the T06 content
//! migration). This test therefore points a real `AssetServer` at a TEST fixture root
//! (`tests/fixtures/new_terrain_root/`) whose `terrain/industrial_hive/` holds one
//! `*.terrain_def.ron` + one `*.terrain_theme.ron`, with the OTHER shipped content dirs
//! symlinked to the real `assets/` so the rest of the Load gate still clears and the new
//! resolve branch actually runs end-to-end.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts the registries POPULATE and resolve the KNOWN authored
//! UUIDs only — no authored magnitudes pinned. Mirrors `load_terrain.rs` /
//! `load_themes.rs` style.

use std::path::PathBuf;

use gdtf_app::test_support::AppState;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an async load
/// resolving. The per-theme `terrain/` folder load shares the `AssetServer` with every other
/// loaded folder, so under parallel `cargo` contention the async resolve has NO fixed frame
/// count. These waits key off the resolved SIGNAL; the cap is a safety net against a genuine
/// never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The KNOWN terrain-def UUID authored in `deck.terrain_def.ron`
/// (`Uuid::from_u128(0x0184_0a3e_0801)`).
const fn known_terrain_uuid() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0801))
}

/// The KNOWN theme-def UUID authored in `industrial_hive.terrain_theme.ron`
/// (`Uuid::from_u128(0x0184_0a3e_08a1)`).
const fn known_theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_08a1))
}

/// The absolute path of the per-theme TEST fixture root (this crate's
/// `tests/fixtures/new_terrain_root/`), computed lexically from the manifest dir so it is
/// independent of the cwd.
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("new_terrain_root")
}

/// Reads the current [`AppState`].
fn app_state(app: &bevy::app::App) -> AppState {
    app.world()
        .resource::<bevy::state::state::State<AppState>>()
        .get()
        .clone()
}

/// GTW-487 C1 — with a real `AssetServer` rooted at the per-theme TEST fixture, entering
/// `Load` loads `terrain/industrial_hive/*.terrain_def.ron` + `*.terrain_theme.ron` through
/// the ACTUAL `resolve_terrain_defs` / `resolve_theme_defs` branch and builds the UUID-keyed
/// registries. Proves:
///
/// - The [`TerrainDefRegistry`] RESOLVES POPULATED (non-empty) and resolves the KNOWN
///   authored [`TerrainUuid`] — keyed by the def's OWN UUID, not the filename.
/// - The [`UuidThemeRegistry`] RESOLVES POPULATED (non-empty) and resolves the KNOWN authored
///   [`ThemeUuid`] — and its `default_floor` resolves to the known terrain UUID.
/// - The Load gate WAITED for both: the machine reaches `Intro` with both present, proving
///   the GTW-487 gate clause fired on real registries (not hand-seeded defaults).
///
/// DELIBERATELY does NOT seed either registry — existence here proves the REAL resolve branch
/// published them from the fixture folder. VALUE-AGNOSTIC: presence + known-UUID resolution
/// only, no authored magnitudes.
#[test]
fn real_asset_resolves_new_terrain_and_theme_registries_by_uuid() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(fixture_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async per-theme `terrain/` folder load until BOTH new registries are
    // inserted by the real resolve branch. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<TerrainDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    // --- TerrainDefRegistry: non-empty + resolves the known authored UUID ----------------
    if let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved TerrainDefRegistry must carry the authored fixture def(s)",
        );
        assert!(
            registry.def(&known_terrain_uuid()).is_some(),
            "the registry must resolve the known authored TerrainUuid (keyed by the def's own \
             UUID, not the filename)",
        );
    }

    // --- UuidThemeRegistry: non-empty + resolves the known authored UUID ------------------
    if let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved UuidThemeRegistry must carry the authored fixture theme def(s)",
        );
        assert!(
            registry.def(&known_theme_uuid()).is_some(),
            "the registry must resolve the known authored ThemeUuid (keyed by the def's own \
             UUID)",
        );
        // The authored theme's default_floor references the known terrain UUID — proving the
        // cross-reference round-trips through the loader.
        assert_eq!(
            registry.default_floor(&known_theme_uuid()),
            Some(known_terrain_uuid()),
            "the resolved theme's default_floor must reference the authored terrain UUID",
        );
    }

    // --- Load gate WAITED for both new registries -----------------------------------------
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once every folder (incl. the new \
         per-theme terrain/) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<TerrainDefRegistry>().is_some()
            && app.world().get_resource::<UuidThemeRegistry>().is_some(),
        "both new registries must be present when Load reaches Intro (the GTW-487 gate clause \
         waited for them — not hand-seeded defaults)",
    );
}
