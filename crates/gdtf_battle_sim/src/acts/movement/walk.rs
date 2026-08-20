//! Multi-step walk: one step per frame until the route ends or something stops it.

use bevy::{
    ecs::{
        query::QueryData,
        system::{ParamSet, SystemParam},
    },
    platform::collections::HashSet,
    prelude::{Commands, Deref, Entity, Message, MessageReader, MessageWriter, Query, Res},
};

use super::params::MoveCommit;
use crate::{
    acts::{MoveCompleted, MovementOccurred},
    ganger::{Faction, LifeState, Position, Tu},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    terrain::emplacement::Mounted,
    tu::spend_tu,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

/// Whether the walk route has no steps left.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteComplete(bool);

impl RouteComplete {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(complete: bool) -> Self {
        Self(complete)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct NewEnemyRevealed(bool);

impl NewEnemyRevealed {
    const fn new(revealed: bool) -> Self {
        Self(revealed)
    }
}

/// Reaction fire interrupted this mover's walk.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReactionShotFired {
    /// Entity that was walking.
    pub mover: Entity,
}

impl ReactionShotFired {
    /// Build an interrupt message.
    #[must_use]
    pub const fn new(mover: Entity) -> Self {
        Self { mover }
    }
}

/// Whether a walk has popped a cell yet.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkStepped(bool);

impl WalkStepped {
    /// Wrap a stepped flag.
    #[must_use]
    pub const fn new(stepped: bool) -> Self {
        Self(stepped)
    }
}

/// In-progress multi-cell walk with remaining costs and enemy-reveal baseline.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct WalkInProgress {
    remaining_cells: Vec<CellLevel>,
    remaining_costs: Vec<Tu>,
    seen_enemies:    Option<HashSet<CellLevel>>,
    stepped:         WalkStepped,
}

impl WalkInProgress {
    /// Build from path cells and per-step costs (stored reversed for pop).
    #[must_use]
    pub fn new(remaining_cells: &[CellLevel], remaining_costs: &[Tu]) -> Self {
        let mut cells: Vec<CellLevel> = remaining_cells.to_vec();
        let mut costs: Vec<Tu> = remaining_costs.to_vec();
        cells.reverse();
        costs.reverse();
        Self {
            remaining_cells: cells,
            remaining_costs: costs,
            seen_enemies:    None,
            stepped:         WalkStepped::new(false),
        }
    }

    /// Whether this walk has ever moved the mover off its start cell.
    #[must_use]
    pub const fn has_stepped(&self) -> WalkStepped {
        self.stepped
    }

    /// True when no steps remain.
    #[must_use]
    pub const fn is_complete(&self) -> RouteComplete {
        RouteComplete::new(self.remaining_cells.is_empty())
    }

    #[must_use]
    fn peek_next(&self) -> Option<(CellLevel, Tu)> {
        let cell = self.remaining_cells.last().copied()?;
        let cost = self.remaining_costs.last().copied()?;
        Some((cell, cost))
    }

    fn pop_next(&mut self) {
        self.remaining_cells.pop();
        self.remaining_costs.pop();
        self.stepped = WalkStepped::new(true);
    }

    fn reveals_new_enemy(&mut self, current: &HashSet<CellLevel>) -> NewEnemyRevealed {
        match &self.seen_enemies {
            None => {
                self.seen_enemies = Some(current.clone());
                NewEnemyRevealed::new(false)
            }
            Some(baseline) => {
                NewEnemyRevealed::new(current.iter().any(|cell| !baseline.contains(cell)))
            }
        }
    }
}

fn visible_enemy_cells(
    mover: Entity,
    mover_faction: Faction,
    gangers: &[(Entity, CellLevel, Faction)],
    squad: &SquadVisibility,
) -> HashSet<CellLevel> {
    gangers
        .iter()
        .filter(|(entity, ..)| *entity != mover)
        .filter_map(|(_, cell, faction)| {
            let relation = if *faction == mover_faction {
                FactionRelation::OwnSquad
            } else {
                FactionRelation::Other
            };
            if relation == FactionRelation::Other && *is_ganger_visible(squad, cell, relation) {
                Some(*cell)
            } else {
                None
            }
        })
        .collect()
}

/// One walking ganger's columns: where it is, and what it has left to walk.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct WalkerRow {
    /// The walking ganger.
    pub entity:   Entity,
    /// Current cell, advanced one step per frame.
    pub position: &'static mut Position,
    /// Remaining time units, spent per step.
    pub tu:       &'static mut Tu,
    /// Alive check — a dead walker stops.
    pub life:     &'static LifeState,
    /// Faction, used to classify revealed gangers.
    pub faction:  &'static Faction,
    /// The seat this walker still rides, cleared by its first step off it.
    pub mounted:  Option<&'static Mounted>,
    /// Route and enemy-reveal baseline.
    pub walk:     &'static mut WalkInProgress,
}

// Query over every walking ganger.
pub(super) type WalkerQuery<'world, 'state> = Query<'world, 'state, WalkerRow>;

// Query of every ganger's cell and faction, for the reveal check.
pub(super) type GangerCellQuery<'world, 'state> =
    Query<'world, 'state, (Entity, &'static Position, &'static Faction)>;

// The walkers themselves, plus the cell snapshot they check for enemy reveals.
pub(super) type WalkWorld<'world, 'state> = ParamSet<
    'world,
    'state,
    (
        WalkerQuery<'static, 'static>,
        GangerCellQuery<'static, 'static>,
    ),
>;

/// Advance one step per frame for each walking ganger.
///
/// The walk stops when the mover is no longer alive, a reaction shot interrupts it, the step
/// reveals a new enemy, the route runs out, the next cell is blocked or occupied, the pool
/// cannot pay for the step, or the walk completes. A mounted walker gives its seat up on the
/// first step it takes off it, so a walk that ends before stepping keeps the seat.
pub fn advance_walk(
    mut world: WalkWorld,
    grid: Res<OccupancyGrid>,
    squad: Res<SquadVisibility>,
    mut reactions: MessageReader<ReactionShotFired>,
    mut signals: WalkSignals,
    mut commit: MoveCommit,
    mut commands: Commands,
) {
    let interrupted: HashSet<Entity> = reactions.read().map(|shot| shot.mover).collect();

    let snapshot: Vec<(Entity, CellLevel, Faction)> = world
        .p1()
        .iter()
        .map(|(entity, position, faction)| (entity, **position, *faction))
        .collect();

    for row in &mut world.p0() {
        let mover = row.entity;
        let life = *row.life;
        let faction = *row.faction;
        let mounted = row.mounted;
        let mut position = row.position;
        let mut tu = row.tu;
        let mut walk = row.walk;
        if !matches!(life, LifeState::Alive) {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        }

        if interrupted.contains(&mover) {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        }

        let visible_now = visible_enemy_cells(mover, faction, &snapshot, &squad);
        if *walk.reveals_new_enemy(&visible_now) {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        }

        let Some((next, cost)) = walk.peek_next() else {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        };

        if *grid.is_path_blocked(&next) || grid.occupant(&next).is_some() {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        }

        if spend_tu(&mut tu, cost).is_err() {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
            continue;
        }
        let from = position.cell();
        *position = Position::new(next);
        signals
            .steps
            .write(MovementOccurred::new(mover, from, next.cell()));
        commit.vacate(mounted);
        walk.pop_next();

        if *walk.is_complete() {
            end_walk(&mut commands, &mut signals, mover, &walk, *position);
        }
    }
}

/// Both movement buffers a walk writes to, as one bag.
#[derive(SystemParam)]
pub struct WalkSignals<'w> {
    steps: MessageWriter<'w, MovementOccurred>,
    moves: MessageWriter<'w, MoveCompleted>,
}

// Drop the walk, announcing a completed move only when the walk ever left its start cell.
fn end_walk(
    commands: &mut Commands,
    signals: &mut WalkSignals,
    mover: Entity,
    walk: &WalkInProgress,
    at: Position,
) {
    commands.entity(mover).remove::<WalkInProgress>();
    if *walk.has_stepped() {
        signals.moves.write(MoveCompleted::new(mover, *at));
    }
}
