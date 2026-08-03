use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level},
    weapon::FireModeSpec,
};

#[derive(Message, Debug, Clone, PartialEq)]
pub struct FireRequested {
        pub shooter:      Entity,
            pub mode:         FireModeSpec,
        pub target_cell:  Cell,
        pub target_level: Level,
}

impl FireRequested {
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
