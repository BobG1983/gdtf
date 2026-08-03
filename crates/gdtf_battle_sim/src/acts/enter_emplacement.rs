use bevy::prelude::{MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::{
        downed::is_8_adjacent,
        request::{EnterEmplacementRequested, ExitEmplacementRequested},
    },
    ganger::{Position, Tu},
    terrain::{
        emplacement::{EmplacementOccupant, EmplacementState, SetEmplacement},
        entity::TerrainCell,
    },
    tu::spend_tu,
    tuning::CombatTuning,
};

pub fn dispatch_enter_emplacement(
    mut requests: MessageReader<EnterEmplacementRequested>,
    emplacements: Query<(&EmplacementState, &TerrainCell)>,
    mut actors: Query<(&Position, &mut Tu)>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        let Ok((state, cell)) = emplacements.get(request.emplacement) else {
            continue;
        };
        if *state.is_occupied() {
            continue;
        }
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.enter_emplacement_tu);
        if !*is_8_adjacent(actor_pos, Position::new(**cell)) || **actor_tu < *cost {
            continue;
        }
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetEmplacement::occupy(request.emplacement, request.actor));
    }
}

pub fn dispatch_exit_emplacement(
    mut requests: MessageReader<ExitEmplacementRequested>,
    emplacements: Query<(&EmplacementState, &EmplacementOccupant)>,
    mut actors: Query<&mut Tu>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        let Ok((state, occupant)) = emplacements.get(request.emplacement) else {
            continue;
        };
        if !*state.is_occupied() || **occupant != request.actor {
            continue;
        }
        let Ok(mut actor_tu) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.exit_emplacement_tu);
        if **actor_tu < *cost {
            continue;
        }
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetEmplacement::vacate(request.emplacement, request.actor));
    }
}
