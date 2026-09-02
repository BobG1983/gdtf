//! Authored situation value and the setup that spawns it into a live battle.

mod error;
mod setup;
mod spawn;
pub(crate) mod terrain_resolve;

#[cfg(test)]
mod test;

pub use error::BattleSetupError;
pub use setup::{
    BattleRegistries, BattleSetup, GangerCount, StackedGangers, has_stacked_gangers, setup_battle,
};
pub use spawn::{
    CoverSpawn, FieldSpawn, FloorSpawn, GangerSpawn, PlacedGanger, Placement, RosterMember,
    Situation, SlabSpawn,
};
