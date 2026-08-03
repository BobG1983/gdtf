use bevy::prelude::{MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::{downed::is_8_adjacent, request::OpenDoorRequested},
    ganger::{Position, Tu},
    terrain::{
        entity::TerrainCell,
        openable::{OpenState, SetOpenable},
    },
    tu::spend_tu,
    tuning::CombatTuning,
};

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
        if *open_state.is_open() {
            continue;
        }
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.open_door_tu);
        if !*is_8_adjacent(actor_pos, Position::new(**door_cell)) || **actor_tu < *cost {
            continue;
        }
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetOpenable::toggle(request.door, *open_state));
    }
}
