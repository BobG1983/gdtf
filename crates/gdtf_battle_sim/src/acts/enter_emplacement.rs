//! Enter and exit weapon emplacements.

use bevy::prelude::{Deref, Entity, MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::request::{EnterEmplacementRequested, ExitEmplacementRequested},
    ganger::{Position, Tu},
    metric::{Cell, CellLevel, Level},
    terrain::{
        def::rotated_entry_sides,
        emplacement::{
            EmplacementEntrySides, EmplacementFacing, EmplacementOccupant, EmplacementState,
            SetEmplacement,
        },
        entity::TerrainCell,
        facing::TerrainFacing,
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
    let Some(sides) = sides else {
        return OnEntrySide::new(false);
    };
    let turned = facing.map_or_else(TerrainFacing::default, |placed| **placed);
    let (seat, level) = emplacement.split();
    OnEntrySide::new(
        rotated_entry_sides(sides, turned)
            .into_iter()
            .any(|side| entry_cell(seat, level, side.cell_step()) == actor),
    )
}

fn entry_cell(seat: Cell, level: Level, step: Cell) -> Position {
    Position::new(CellLevel::new(
        Cell::new(seat.x + step.x, seat.y + step.y),
        level,
    ))
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

/// System: occupy an empty emplacement from one of its rotated entry sides.
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
    for request in requests.read() {
        let Ok((state, cell, sides, facing)) = emplacements.get(request.emplacement) else {
            continue;
        };
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let seat = Position::new(**cell);
        if !*can_enter_emplacement(actor_pos, seat, state, sides, facing, &actor_tu, &tuning) {
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
