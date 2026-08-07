//! Dispatch open-door requests for adjacent closed doors.

use bevy::prelude::{Deref, MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::{downed::is_8_adjacent, request::OpenDoorRequested},
    ganger::{Position, Tu},
    terrain::{
        entity::TerrainCell,
        openable::{OpenState, SetOpenable},
    },
    tu::{can_spend_tu, spend_tu},
    tuning::CombatTuning,
};

/// Whether the actor may open this door right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanOpenDoor(bool);

impl CanOpenDoor {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for opening one door.
#[must_use]
pub fn open_door_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.open_door_tu)
}

/// Door is closed, the actor is adjacent, and the pool covers the cost.
#[must_use]
pub fn can_open_door(
    actor: Position,
    door: Position,
    open_state: OpenState,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanOpenDoor {
    CanOpenDoor::new(
        !*open_state.is_open()
            && *is_8_adjacent(actor, door)
            && *can_spend_tu(tu, open_door_tu_cost(tuning)),
    )
}

/// System: open adjacent closed doors when TU allows.
pub fn dispatch_open_door(
    mut requests: MessageReader<OpenDoorRequested>,
    doors: Query<(&OpenState, &TerrainCell)>,
    mut actors: Query<(&Position, &mut Tu)>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetOpenable>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        let Ok((open_state, door_cell)) = doors.get(request.door) else {
            continue;
        };
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let door_pos = Position::new(**door_cell);
        if !*can_open_door(actor_pos, door_pos, *open_state, &actor_tu, &tuning) {
            continue;
        }
        spend_tu(&mut actor_tu, open_door_tu_cost(&tuning));
        toggles.write(SetOpenable::toggle(request.door, *open_state));
    }
}
