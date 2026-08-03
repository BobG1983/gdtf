use bevy::prelude::*;
use gdtf_battle_sim::prelude::{Cell, Level, SimPos};

pub const CELL_PX: f32 = 16.0;

const Z_PER_LEVEL: f32 = 1.0;

pub const GANGER_Z_BIAS: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
        Terrain,
                                Field,
                        VerticalLink,
                        FireTarget,
        Actor,
                        CrossLevelSignal,
            Highlight,
                    ReachableRange,
                        PathPreview,
}

impl Layer {
                        #[must_use]
    const fn z_bias(self) -> f32 {
        match self {
            Self::Terrain => 0.0,
            Self::Field => GANGER_Z_BIAS * 0.25,
            Self::VerticalLink => GANGER_Z_BIAS * 0.5,
            Self::FireTarget => GANGER_Z_BIAS * 0.75,
            Self::Actor => GANGER_Z_BIAS,
            Self::CrossLevelSignal => GANGER_Z_BIAS * 1.5,
            Self::Highlight => GANGER_Z_BIAS * 2.0,
            Self::ReachableRange => GANGER_Z_BIAS * 2.5,
            Self::PathPreview => GANGER_Z_BIAS * 3.0,
        }
    }
}

#[must_use]
pub fn cell_to_world(cell: Cell, level: Level) -> Vec3 {
    Vec3::new(
        cell.x as f32 * CELL_PX,
        -(cell.y as f32) * CELL_PX,
        z_for(level),
    )
}

#[must_use]
pub fn sim_pos_to_world(pos: SimPos) -> Vec3 {
    Vec3::new(pos.x * CELL_PX, -pos.y * CELL_PX, pos.z * Z_PER_LEVEL)
}

#[must_use]
pub fn cell_to_world_layered(cell: Cell, level: Level, layer: Layer) -> Vec3 {
    let mut world = cell_to_world(cell, level);
    world.z += layer.z_bias();
    world
}

pub(super) fn z_for(level: Level) -> f32 {
    f32::from(*level) * Z_PER_LEVEL
}
