mod components;
mod plugin;
mod systems;

#[cfg(test)]
mod test;

pub use components::{GroundAccrued, PrevSlot, TerrainPieceDestroyed};
pub use plugin::{OccupancyMaintenancePlugin, SimSystems};
pub use systems::{sync_accrued_ground, sync_dead_gangers, sync_moved_gangers};
