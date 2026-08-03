//! Messages and systems that open or close openable terrain.

use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Update,
};

use super::{OpenState, OpenableBlocking};
use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::SimSystems,
    terrain::entity::{BlocksPathfinding, BlocksVision},
};

/// Request to set an openable to a specific state.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetOpenable {
    entity: Entity,
    state: OpenState,
}

impl SetOpenable {
    /// Build a request for `entity` to become `state`.
    #[must_use]
    pub const fn new(entity: Entity, state: OpenState) -> Self {
        Self { entity, state }
    }

    /// Open the entity.
    #[must_use]
    pub const fn open(entity: Entity) -> Self {
        Self::new(entity, OpenState::Open)
    }

    /// Close the entity.
    #[must_use]
    pub const fn close(entity: Entity) -> Self {
        Self::new(entity, OpenState::Closed)
    }

    /// Toggle from the current state.
    #[must_use]
    pub const fn toggle(entity: Entity, current: OpenState) -> Self {
        Self::new(entity, current.toggled())
    }

    /// Target entity.
    #[must_use]
    pub const fn entity(self) -> Entity {
        self.entity
    }

    /// Requested open state.
    #[must_use]
    pub const fn state(self) -> OpenState {
        self.state
    }
}

/// Apply open/close requests: update state and path/vision blocking components.
pub fn apply_openable_toggle(
    mut requests: MessageReader<SetOpenable>,
    mut doors: Query<(&mut OpenState, &OpenableBlocking)>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((mut open_state, blocking)) = doors.get_mut(request.entity()) else {
            continue;
        };
        if *open_state == request.state() {
            continue;
        }
        *open_state = request.state();
        let mut entity = commands.entity(request.entity());
        if *request.state().is_open() {
            entity.remove::<BlocksPathfinding>();
            entity.remove::<BlocksVision>();
        } else {
            entity.insert(BlocksPathfinding);
            entity.insert(BlocksVision::new(**blocking));
        }
    }
}

/// Plugin that registers the openable toggle system.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenableTogglePlugin;

impl Plugin for OpenableTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetOpenable>().add_systems(
            Update,
            apply_openable_toggle
                .in_set(SimSystems::Simulate)
                .before(project_path_blocking)
                .before(project_vision_blocking),
        );
    }
}
