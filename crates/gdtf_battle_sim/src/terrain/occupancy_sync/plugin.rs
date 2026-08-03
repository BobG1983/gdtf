use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet, Update};

use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::{
        CoverDestroyed, GroundAccrued, SlabDestroyed, sync_accrued_ground, sync_dead_gangers,
        sync_destroyed_cover, sync_destroyed_slab, sync_moved_gangers,
    },
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSystems {
            Simulate,
                            Record,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OccupancyMaintenancePlugin;

impl Plugin for OccupancyMaintenancePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CoverDestroyed>()
            .add_message::<SlabDestroyed>()
            .add_message::<GroundAccrued>()
            .configure_sets(Update, SimSystems::Simulate)
            .configure_sets(Update, SimSystems::Record.after(SimSystems::Simulate))
            .add_systems(
                Update,
                (
                    sync_moved_gangers,
                    sync_dead_gangers,
                    sync_destroyed_cover,
                    project_path_blocking,
                    project_vision_blocking,
                )
                    .chain()
                    .in_set(SimSystems::Simulate),
            )
            .add_systems(Update, sync_destroyed_slab.in_set(SimSystems::Simulate))
            .add_systems(
                Update,
                sync_accrued_ground
                    .after(sync_destroyed_slab)
                    .in_set(SimSystems::Simulate),
            );
    }
}
