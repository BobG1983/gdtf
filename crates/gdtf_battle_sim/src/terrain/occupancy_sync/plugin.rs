//! Plugin that keeps occupancy and surface grids in sync.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet, Update};

use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::{
        GroundAccrued, TerrainPieceDestroyed, sync_accrued_ground, sync_dead_gangers,
        sync_moved_gangers,
    },
    terrain::{
        emplacement::eject_on_destroy,
        successor::{SlabLeftOpen, despawn_replaced_piece, replace_destroyed_piece},
    },
};

/// System sets for occupancy maintenance.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSystems {
    /// Project moves, deaths, and blocking onto the grid.
    Simulate,
    /// Record after simulate.
    Record,
}

/// Registers occupancy sync systems and messages.
#[derive(Debug, Default, Clone, Copy)]
pub struct OccupancyMaintenancePlugin;

impl Plugin for OccupancyMaintenancePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TerrainPieceDestroyed>()
            .add_message::<GroundAccrued>()
            .add_message::<SlabLeftOpen>()
            .configure_sets(Update, SimSystems::Simulate)
            .configure_sets(Update, SimSystems::Record.after(SimSystems::Simulate))
            .add_systems(
                Update,
                (
                    sync_moved_gangers,
                    sync_dead_gangers,
                    replace_destroyed_piece,
                    project_path_blocking,
                    project_vision_blocking,
                )
                    .chain()
                    .in_set(SimSystems::Simulate),
            )
            .add_systems(
                Update,
                sync_accrued_ground
                    .after(replace_destroyed_piece)
                    .in_set(SimSystems::Simulate),
            )
            .add_systems(
                Update,
                despawn_replaced_piece
                    .after(replace_destroyed_piece)
                    .after(eject_on_destroy)
                    .in_set(SimSystems::Simulate),
            );
    }
}
