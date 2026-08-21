//! Enter and exit weapon emplacements.

use bevy::{
    ecs::relationship::Relationship,
    prelude::{Deref, Entity, MessageReader, MessageWriter, Query, Res},
};

use crate::{
    acts::{
        pending_state::PendingStates,
        request::{EnterEmplacementRequested, ExitEmplacementRequested},
    },
    ganger::{Position, Tu},
    occupancy::OccupancyGrid,
    terrain::{
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementState, EnteredFrom, MountedBy,
            SetEmplacement, emplacement_entry_cells,
        },
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

// Whether the actor stands on one of an emplacement's rotated entry sides.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct OnEntrySide(bool);

impl OnEntrySide {
    // Wrap a boolean.
    const fn new(standing: bool) -> Self {
        Self(standing)
    }
}

// An emplacement naming no side has no entry cell; an absent facing reads as the default one.
fn on_an_entry_side(
    actor: Position,
    emplacement: Position,
    sides: Option<&EmplacementEntrySides>,
    facing: Option<&EmplacementFacing>,
) -> OnEntrySide {
    OnEntrySide::new(emplacement_entry_cells(*emplacement, sides, facing).contains(&actor))
}

/// Emplacement is empty, the actor stands on one of its rotated entry sides, and the pool pays.
#[must_use]
pub fn can_enter_emplacement(
    actor: Position,
    emplacement: Position,
    state: &EmplacementState,
    sides: Option<&EmplacementEntrySides>,
    facing: Option<&EmplacementFacing>,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanEnterEmplacement {
    CanEnterEmplacement::new(
        !*state.is_occupied()
            && *on_an_entry_side(actor, emplacement, sides, facing)
            && *can_spend_tu(tu, enter_emplacement_tu_cost(tuning)),
    )
}

// Whether the cell the occupant entered from is free for it to be put back on.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct EntryCellFree(bool);

impl EntryCellFree {
    // Wrap a boolean.
    const fn new(free: bool) -> Self {
        Self(free)
    }
}

// A seat holding no remembered cell has none to test, so nothing stands in the way.
fn remembered_cell_is_free(entered: Option<&EnteredFrom>, grid: &OccupancyGrid) -> EntryCellFree {
    EntryCellFree::new(entered.is_none_or(|from| grid.occupant(from).is_none()))
}

/// The actor holds this emplacement, the cell it entered from is free, and the pool pays.
#[must_use]
pub fn can_exit_emplacement(
    actor: Entity,
    state: &EmplacementState,
    occupant: &MountedBy,
    entered: Option<&EnteredFrom>,
    grid: &OccupancyGrid,
    tu: &Tu,
    tuning: &CombatTuning,
) -> CanExitEmplacement {
    CanExitEmplacement::new(
        *state.is_occupied()
            && occupant.get() == actor
            && *remembered_cell_is_free(entered, grid)
            && *can_spend_tu(tu, exit_emplacement_tu_cost(tuning)),
    )
}

/// System: occupy an empty emplacement from one of its rotated entry sides.
/// A repeat request in the same frame reads the seat this run already manned, so it pays nothing.
pub fn dispatch_enter_emplacement(
    mut requests: MessageReader<EnterEmplacementRequested>,
    emplacements: Query<(
        &EmplacementState,
        &TerrainCell,
        Option<&EmplacementEntrySides>,
        Option<&EmplacementFacing>,
    )>,
    mut actors: Query<(&Position, &mut Tu)>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    let mut pending: PendingStates<EmplacementState> = PendingStates::new();
    for request in requests.read() {
        let Ok((state, cell, sides, facing)) = emplacements.get(request.emplacement) else {
            continue;
        };
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let seat = Position::new(**cell);
        let state = pending.state_of(request.emplacement, *state);
        if !*can_enter_emplacement(actor_pos, seat, &state, sides, facing, &actor_tu, &tuning) {
            continue;
        }
        if spend_tu(&mut actor_tu, enter_emplacement_tu_cost(&tuning)).is_err() {
            continue;
        }
        pending.record(request.emplacement, EmplacementState::Occupied);
        toggles.write(SetEmplacement::occupy(request.emplacement, request.actor));
    }
}

/// System: vacate an emplacement the actor currently occupies.
/// A repeat request in the same frame reads the seat this run already vacated, so it pays nothing.
pub fn dispatch_exit_emplacement(
    mut requests: MessageReader<ExitEmplacementRequested>,
    emplacements: Query<(&EmplacementState, &MountedBy, Option<&EnteredFrom>)>,
    mut actors: Query<&mut Tu>,
    grid: Res<OccupancyGrid>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    let mut pending: PendingStates<EmplacementState> = PendingStates::new();
    for request in requests.read() {
        let Ok((state, occupant, entered)) = emplacements.get(request.emplacement) else {
            continue;
        };
        let Ok(mut actor_tu) = actors.get_mut(request.actor) else {
            continue;
        };
        let state = pending.state_of(request.emplacement, *state);
        if !*can_exit_emplacement(
            request.actor,
            &state,
            occupant,
            entered,
            &grid,
            &actor_tu,
            &tuning,
        ) {
            continue;
        }
        if spend_tu(&mut actor_tu, exit_emplacement_tu_cost(&tuning)).is_err() {
            continue;
        }
        pending.record(request.emplacement, EmplacementState::Vacant);
        toggles.write(SetEmplacement::vacate(request.emplacement, request.actor));
    }
}
