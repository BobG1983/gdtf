mod components;
mod plugin;
mod systems;

#[cfg(test)]
mod test;

pub use components::{CoverDestroyed, GroundAccrued, PrevSlot, SlabDestroyed};
pub use plugin::{OccupancyMaintenancePlugin, SimSystems};
pub use systems::{
    sync_accrued_ground, sync_dead_gangers, sync_destroyed_cover, sync_destroyed_slab,
    sync_moved_gangers,
};
