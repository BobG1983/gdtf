use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Update,
};

use super::{OpenState, OpenableBlocking};
use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::SimSystems,
    terrain::entity::{BlocksPathfinding, BlocksVision},
};

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetOpenable {
        entity: Entity,
            state:  OpenState,
}

impl SetOpenable {
        #[must_use]
    pub const fn new(entity: Entity, state: OpenState) -> Self {
        Self { entity, state }
    }

        #[must_use]
    pub const fn open(entity: Entity) -> Self {
        Self::new(entity, OpenState::Open)
    }

        #[must_use]
    pub const fn close(entity: Entity) -> Self {
        Self::new(entity, OpenState::Closed)
    }

                #[must_use]
    pub const fn toggle(entity: Entity, current: OpenState) -> Self {
        Self::new(entity, current.toggled())
    }

        #[must_use]
    pub const fn entity(self) -> Entity {
        self.entity
    }

        #[must_use]
    pub const fn state(self) -> OpenState {
        self.state
    }
}

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
