//! GTW-490 C5: with a REAL `AssetServer` rooted at the SHIPPED workspace `assets/`, entering
//! `AppState::Load` resolves `content/situations/skirmish.ron` through the actual GTW-205/261 situation
//! loader into the persistent [`LoadedSituation`], and that situation NAMES a migrated
//! [`ThemeUuid`] (its `theme_uuid` field) that RESOLVES in the migrated
//! [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry).
//!
//! This proves the migrated skirmish theme reference round-trips end-to-end THROUGH the
//! situation loader path (not a hand-inserted resource): `skirmish.ron` parses, its new
//! `theme_uuid` is the `IndustrialHive` `ThemeUuid`, and that UUID is one the migrated theme
//! registry resolves. The legacy `theme: IndustrialHive` enum is untouched (the live path
//! still reads it — C7 / scope boundary), so this is purely the NEW model resolving alongside.
//!
//! VALUE-AGNOSTIC (gate 4a / C6): identity / resolution / consistency only — no magnitude pinned.

use gdtf_app::test_support::{AppState, LoadedSituation, app_state, load_released};
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset waits gated on an async load resolving (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] `skirmish.ron` authors in its `theme_uuid` field
/// (matches the migrated `industrial_hive.terrain_theme.ron` key).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// GTW-490 C5 — `skirmish.ron` resolves through the situation loader, names the migrated
/// `IndustrialHive` [`ThemeUuid`], and that UUID resolves in the migrated [`UuidThemeRegistry`].
#[test]
fn shipped_skirmish_names_a_migrated_theme_uuid_that_resolves() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<LoadedSituation>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    // GTW-491: the canonical `theme` field IS the migrated IndustrialHive `ThemeUuid` now (the
    // GTW-490 additive `theme_uuid` was reconciled into `theme`). Read it off the resolved
    // situation (the situation loader path).
    let authored_theme_uuid = app
        .world()
        .get_resource::<LoadedSituation>()
        .map(|loaded| loaded.theme);

    assert_eq!(
        authored_theme_uuid,
        Some(industrial_hive_theme()),
        "skirmish.ron must author the migrated IndustrialHive theme (a ThemeUuid), resolved \
         through the situation loader path (C5)",
    );

    // That authored UUID must resolve in the migrated theme registry — the round-trip C5 proves.
    if let (Some(theme_uuid), Some(registry)) = (
        authored_theme_uuid,
        app.world().get_resource::<UuidThemeRegistry>(),
    ) {
        assert!(
            registry.def(&theme_uuid).is_some(),
            "the migrated theme skirmish.ron names must resolve in the UuidThemeRegistry (C5)",
        );
    }

    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once the situation + \
         theme registry resolve; last AppState was {:?}",
        app_state(&app),
    );
}
