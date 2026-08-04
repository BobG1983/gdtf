//! Request to fire a weapon mode at a cell.

use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level},
    weapon::FireModeSpec,
};

/// Fire request from UI/AI.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct FireRequested {
    /// Shooter entity.
    pub shooter:      Entity,
    /// Fire mode to use.
    pub mode:         FireModeSpec,
    /// Target cell.
    pub target_cell:  Cell,
    /// Target level.
    pub target_level: Level,
}

impl FireRequested {
    /// Build a fire request.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        mode: FireModeSpec,
        target_cell: Cell,
        target_level: Level,
    ) -> Self {
        Self {
            shooter,
            mode,
            target_cell,
            target_level,
        }
    }
}
