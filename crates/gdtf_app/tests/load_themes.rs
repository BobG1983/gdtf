//! GTW-487 / GTW-494 / GTW-580: the UUID-keyed theme-defs family's load
//! coverage — the thin wrapper over the generic per-family suite
//! (`load_suite::suite`), plus the family-bespoke `default_floor`
//! cross-reference pin.
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`ThemeDefsFamily`] with the KNOWN authored [`ThemeUuid`].
//! The family is PAYLOAD-KEYED and shares the MIXED `content/terrain/` tree
//! with the terrain defs. VALUE-AGNOSTIC: presence + known-UUID resolution +
//! the `default_floor` cross-reference only — no authored magnitudes pinned.

mod load_suite;

use bevy::asset::uuid::Uuid;
use gdtf_app::test_support::AppState;
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::TerrainUuid,
};
use gdtf_content_families::ThemeDefsFamily;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};
use load_suite::suite::{self, FamilyLoadContract};

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a safety net against a genuine never-resolve
/// hang, not a timing budget (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The migrated `IndustrialHive` [`ThemeUuid`] authored in
/// `content/terrain/industrial_hive/industrial_hive.terrain_theme.ron`
/// (`Uuid::from_u128(0x0184_0a90_0001)`).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0184_0a90_0001))
}

/// The migrated `industrial_hive` `deck_floor` [`TerrainUuid`] the theme
/// authors as its `default_floor` (`Uuid::from_u128(0x0184_0a91_0004)`).
const fn industrial_hive_default_floor() -> TerrainUuid {
    TerrainUuid::new(Uuid::from_u128(0x0184_0a91_0004))
}

impl FamilyLoadContract for ThemeDefsFamily {
    /// The migrated `IndustrialHive` theme, pinned by its authored UUID.
    const EXPECTED_MEMBERS: &'static [&'static str] = &["industrial_hive"];

    fn is_empty(registry: &UuidThemeRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &UuidThemeRegistry, label: &str) -> bool {
        // Payload-keyed family: the label names a KNOWN authored UUID.
        match label {
            "industrial_hive" => registry.def(&industrial_hive_theme()).is_some(),
            _ => false,
        }
    }
}

/// Tier (a) — the theme-defs loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn theme_defs_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<ThemeDefsFamily>();
}

/// Tier (a) companion — the Load→Intro transition GATES on the
/// [`UuidThemeRegistry`] (the GTW-487 gate clause: the per-theme terrain
/// folder is verified loaded before `Load` exits).
#[test]
fn load_does_not_leave_without_a_uuid_theme_registry() {
    suite::load_gates_on_registry::<ThemeDefsFamily>();
}

/// Tier (b) / GTW-494 C2 — the REAL `content/terrain/<theme>/*.terrain_theme.ron`
/// files resolve into the UUID-keyed [`UuidThemeRegistry`] through the Load
/// code path (the known authored `IndustrialHive` UUID resolves — the C2
/// contract, via the real Load branch, not a seeded default).
#[test]
fn real_asset_resolves_uuid_theme_registry() {
    suite::real_asset_resolves_registry::<ThemeDefsFamily>();
}

/// Family-bespoke pin — the resolved theme's `default_floor` cross-reference
/// round-trips through the loader: the `IndustrialHive` theme references the
/// known `deck_floor` [`TerrainUuid`]. Beyond the generic member-resolves
/// check, this pins the cross-family UUID reference the procgen theme
/// resolution consumes.
#[test]
fn real_asset_theme_default_floor_cross_reference_resolves() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the async per-theme folder load (cap is a safety net, GTW-305).
    // DELIBERATELY no seeded registry — existence proves the real resolve ran.
    advance_until_resource_exists::<UuidThemeRegistry>(&mut app, LOAD_SAFETY_NET);

    let registry = app.world().get_resource::<UuidThemeRegistry>();
    assert!(
        registry.is_some(),
        "the real per-theme folder load must insert a UuidThemeRegistry within the safety-net \
         budget",
    );
    if let Some(registry) = registry {
        assert_eq!(
            registry.default_floor(&industrial_hive_theme()),
            Some(industrial_hive_default_floor()),
            "the resolved theme's default_floor must reference the authored terrain UUID",
        );
    }
}
