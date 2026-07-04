//! GTW-494 (child T08 of the GTW-476 refactor): `AppState::Load` loads the per-theme
//! `content/terrain/<theme>/*.terrain_theme.ron` files through the GTW-487 `resolve_theme_defs`
//! loader, builds the UUID-keyed [`UuidThemeRegistry`] from them, and gates the Load→Intro
//! transition on it.
//!
//! This file was MIGRATED off the retired flat-dir `resolve_themes` per-file theme model
//! (GTW-409) onto the UUID model: GTW-494 removed the old game-side loader, so the
//! `UuidThemeRegistry` is now the ONLY theme resolver in the Load flow. It does NOT seed the
//! registry — it drives the REAL Load branch over the shipped `assets/content/terrain/` content and
//! asserts the registry POPULATES (not-empty) and a KNOWN authored [`ThemeUuid`] resolves
//! (the C2 contract: not-empty + a known UUID resolves, via the real Load branch, not a
//! seeded default).
//!
//! - **Tier (b)** — `DefaultPlugins` (headless, `backends: None`) via
//!   [`GdtfLoadTestAppBuilder`]: a real `AssetServer` pointed at the workspace `assets/`.
//!   Drives the REAL `AppState::Load` flow; `advance_until_resource_exists::<UuidThemeRegistry>`
//!   signal-polls until `resolve_theme_defs` publishes the registry, then asserts POPULATED.
//!
//! VALUE-AGNOSTIC (gate 4a): asserts presence / known-UUID resolution / `default_floor`
//! cross-reference ONLY — no authored magnitudes pinned.

use gdtf_app::test_support::{AppState, app_state};
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::TerrainUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_ui::theme::GdtfTheme;

/// Generous SAFETY-NET cap for the real-asset Tier (b) `advance_until` waits gated on an
/// async asset load resolving. The per-theme `content/terrain/` folder load shares the `AssetServer`
/// with every other loaded folder, so under parallel `cargo` contention the async resolve has
/// NO fixed frame count. These waits key off the resolved SIGNAL; the cap is a safety net
/// against a genuine never-resolve hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] authored in
/// `content/terrain/industrial_hive/industrial_hive.terrain_theme.ron` (`Uuid::from_u128(0x0184_0a90_0001)`).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// The migrated `industrial_hive` `deck_floor` [`TerrainUuid`] the theme authors as its
/// `default_floor` (`Uuid::from_u128(0x0184_0a91_0004)`).
const fn industrial_hive_default_floor() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a91_0004))
}

/// GTW-494 C2 — with a real `AssetServer` rooted at the workspace `assets/`, entering `Load`
/// loads `content/terrain/<theme>/*.terrain_theme.ron` and builds the UUID-keyed
/// [`UuidThemeRegistry`] through the ACTUAL `resolve_theme_defs` branch. Proves:
///
/// - The registry RESOLVES POPULATED (non-empty).
/// - A KNOWN authored [`ThemeUuid`] (the `IndustrialHive` theme) resolves — keyed by the
///   def's OWN UUID, not the filename.
/// - That theme's `default_floor` cross-reference resolves to the known terrain UUID.
/// - The Load gate WAITED for the registry: the machine reaches Intro with the registry
///   present, proving the gate clause fired on a real registry.
///
/// Does NOT seed [`UuidThemeRegistry::default()`] — seeding would mask the very regression
/// under test. Existence of the resource here proves `resolve_theme_defs` published it from
/// the real folder (not a hand-seeded default).
#[test]
fn real_asset_resolves_uuid_theme_registry() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async per-theme `content/terrain/` folder load: wait until resolve_theme_defs
    // inserts the UuidThemeRegistry, not a fixed frame count. Cap is a safety net (GTW-305).
    // DELIBERATELY do NOT seed UuidThemeRegistry::default() — existence here proves the REAL
    // resolve_theme_defs published it from the assets/content/terrain/ folder.
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<UuidThemeRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved UuidThemeRegistry must carry the migrated (non-empty) theme defs — an \
             empty registry means resolve_theme_defs is broken (fell back to the empty default)",
        );
        assert!(
            registry.def(&industrial_hive_theme()).is_some(),
            "the registry must resolve the known authored IndustrialHive ThemeUuid (keyed by the \
             def's own UUID, not the filename) — the C2 known-UUID-resolves contract",
        );
        // The theme's default_floor cross-reference round-trips through the loader.
        assert_eq!(
            registry.default_floor(&industrial_hive_theme()),
            Some(industrial_hive_default_floor()),
            "the resolved theme's default_floor must reference the authored terrain UUID",
        );
    }

    // --- Load gate WAITED for the registry -----------------------------------------------
    // The machine must reach Intro once every folder (incl. the per-theme terrain/) resolves.
    // The registry is present when it does, proving the gate clause fired on a real registry
    // (not a hand-seeded default).
    let reached_intro = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Intro,
        LOAD_SAFETY_NET,
    );
    assert!(
        reached_intro,
        "with a real AssetServer, Load must reach Intro once all folders (incl. the per-theme \
         terrain/) resolve; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<UuidThemeRegistry>().is_some(),
        "a UuidThemeRegistry must be present when Load reaches Intro \
         (the gate clause waited for it — not a hand-seeded default)",
    );
    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme resolved alongside the theme registry (the gated transition fired)",
    );
}
