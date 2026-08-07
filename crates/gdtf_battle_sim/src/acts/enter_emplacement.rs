//! Enter and exit weapon emplacements.

use bevy::prelude::{Deref, Entity, MessageReader, MessageWriter, Query, Res};

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
    tu::{can_spend_tu, spend_tu},
    tuning::CombatTuning,
};

/// Whether the actor may occupy this emplacement right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEnterEmplacement(bool);

impl CanEnterEmplacement {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// Whether the actor may vacate this emplacement right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanExitEmplacement(bool);

impl CanExitEmplacement {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for entering an emplacement.
#[must_use]
pub fn enter_emplacement_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.enter_emplacement_tu)
}

/// TU charged for leaving an emplacement.
#[must_use]
pub fn exit_emplacement_tu_cost(tuning: &CombatTuning) -> Tu {
    Tu::new(*tuning.exit_emplacement_tu)
}

/// Emplacement is empty, the actor is adjacent, and the pool covers the cost.
#[must_use]
pub fn can_enter_emplacement(
    actor: Position,
    emplacement: Position,
    state: &EmplacementState,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanEnterEmplacement {
    CanEnterEmplacement::new(
        !*state.is_occupied()
            && *is_8_adjacent(actor, emplacement)
            && *can_spend_tu(tu, enter_emplacement_tu_cost(tuning)),
    )
}

/// The actor holds this emplacement and the pool covers the cost.
#[must_use]
pub fn can_exit_emplacement(
    actor: Entity,
    state: &EmplacementState,
    occupant: &EmplacementOccupant,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanExitEmplacement {
    CanExitEmplacement::new(
        *state.is_occupied()
            && **occupant == actor
            && *can_spend_tu(tu, exit_emplacement_tu_cost(tuning)),
    )
}

/// System: occupy an adjacent empty emplacement.
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
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let seat = Position::new(**cell);
        if !*can_enter_emplacement(actor_pos, seat, state, &actor_tu, &tuning) {
            continue;
        }
        if spend_tu(&mut actor_tu, enter_emplacement_tu_cost(&tuning)).is_err() {
            continue;
        }
        toggles.write(SetEmplacement::occupy(request.emplacement, request.actor));
    }
}

/// System: vacate an emplacement the actor currently occupies.
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
        let Ok(mut actor_tu) = actors.get_mut(request.actor) else {
            continue;
        };
        if !*can_exit_emplacement(request.actor, state, occupant, &actor_tu, &tuning) {
            continue;
        }
        if spend_tu(&mut actor_tu, exit_emplacement_tu_cost(&tuning)).is_err() {
            continue;
        }
        toggles.write(SetEmplacement::vacate(request.emplacement, request.actor));
    }
}
