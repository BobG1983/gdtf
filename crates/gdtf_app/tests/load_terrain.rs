//! GTW-487 / GTW-494 / GTW-580: the UUID-keyed terrain-defs family's load
//! coverage — the thin wrapper over the generic per-family suite
//! (`load_suite::suite`).
//!
//! The tier structure (`MinimalPlugins` no-op guard + gate pin, headless
//! real-asset folder resolve) is encoded ONCE in the shared suite; this file
//! only binds it to [`TerrainDefsFamily`] with the KNOWN authored UUIDs. The
//! family is PAYLOAD-KEYED (each def carries its own [`TerrainUuid`]; the
//! filename is irrelevant), so the member labels map to UUID consts here.
//! VALUE-AGNOSTIC: presence + known-UUID resolution only — no authored terrain
//! magnitudes pinned.

mod load_suite;

use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use gdtf_content_families::TerrainDefsFamily;
use load_suite::suite::{self, FamilyLoadContract};

impl FamilyLoadContract for TerrainDefsFamily {
    /// The migrated `industrial_hive` defs pinned by UUID: `deck_floor`
    /// (`0x0184_0a91_0004`) and `bulkhead_wall` (`0x0184_0a91_0002`).
    const EXPECTED_MEMBERS: &'static [&'static str] = &["deck_floor", "bulkhead_wall"];

    fn is_empty(registry: &TerrainDefRegistry) -> bool {
        registry.is_empty()
    }

    fn member_resolves(registry: &TerrainDefRegistry, label: &str) -> bool {
        // Payload-keyed family: the label names a KNOWN authored UUID.
        let uuid = match label {
            "deck_floor" => Uuid::from_u128(0x0184_0a91_0004),
            "bulkhead_wall" => Uuid::from_u128(0x0184_0a91_0002),
            _ => return false,
        };
        registry.def(&TerrainUuid::new(uuid)).is_some()
    }
}

/// AC (tier a) — the terrain-defs loader registration + folder kick-off no-op
/// cleanly under `MinimalPlugins` (bevy-traps rule 1).
#[test]
fn terrain_loader_no_ops_cleanly_without_asset_server() {
    suite::loader_no_ops_without_asset_server::<TerrainDefsFamily>();
}

/// AC (companion, tier a) — the Load→Intro transition GATES on the
/// [`TerrainDefRegistry`] (the GTW-487 gate clause: the per-theme terrain
/// folder is verified loaded before `Load` exits).
#[test]
fn load_does_not_leave_without_a_terrain_def_registry() {
    suite::load_gates_on_registry::<TerrainDefsFamily>();
}

/// AC (tier b) / GTW-494 C2 — the REAL `assets/content/terrain/<theme>/`
/// folder resolves into the UUID-keyed [`TerrainDefRegistry`] through the Load
/// code path (the known authored UUIDs resolve — the C2 contract).
#[test]
fn real_asset_resolves_terrain_def_registry_by_uuid() {
    suite::real_asset_resolves_registry::<TerrainDefsFamily>();
}
