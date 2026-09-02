//! Bevy plugin that runs fall resolution after fire and slab destruction.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update};

use crate::{
    acts::dispatch_fire,
    falls::{FallOccurred, apply_falls},
    occupancy_sync::SimSystems,
    successor::replace_destroyed_piece,
};

/// Registers [`FallOccurred`] and the [`apply_falls`] system.
#[derive(Debug, Default, Clone, Copy)]
pub struct FallsPlugin;

impl Plugin for FallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FallOccurred>().add_systems(
            Update,
            apply_falls
                .after(dispatch_fire)
                .after(replace_destroyed_piece)
                .in_set(SimSystems::Simulate),
        );
    }
}
