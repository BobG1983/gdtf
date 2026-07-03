//! Shared fixtures for the TERRAIN-form in-crate tests.

use gdtf_battle_sim::terrain::def::TerrainUuid;

/// A terrain UUID from a small constant (the tests' minted key) — deterministic, no
/// random UUIDs in tests.
pub(super) fn key() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_0001))
}
