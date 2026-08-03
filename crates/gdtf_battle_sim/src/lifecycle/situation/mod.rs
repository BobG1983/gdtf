//! The canonical authored **situation** and the setup that pours it into the
//! [`Situation`] is the **one canonical** authored battlefield value — it
mod error;
mod setup;
mod spawn;
mod terrain_resolve;

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
