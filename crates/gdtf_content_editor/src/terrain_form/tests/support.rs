use gdtf_battle_sim::terrain::def::TerrainUuid;

pub(super) fn key() -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_0001))
}
