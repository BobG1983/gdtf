//! Multi-step walk: remaining route, reveal stop, reaction interrupt.

use bevy::{
    platform::collections::HashSet,
    prelude::{Commands, Deref, Entity, Message, MessageReader, MessageWriter, Query, Res},
};

use crate::{
    acts::MovementOccurred,
    ganger::{Faction, LifeState, Position, Tu},
    metric::CellLevel,
    occupancy::OccupancyGrid,
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

/// In-progress multi-cell walk with remaining costs and enemy-reveal baseline.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct WalkInProgress {
    remaining_cells: Vec<CellLevel>,
    remaining_costs: Vec<Tu>,
    seen_enemies:    Option<HashSet<CellLevel>>,
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
        }
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

#[expect(
    clippy::type_complexity,
    reason = "the committed walk genuinely needs a ParamSet of the MUTABLE mover query \
              (step + charge + route state) and the READ-ONLY ganger snapshot query \
              (reveal detection) — they overlap on Position (one `&mut`, one `&`), so a \
              ParamSet time-multiplexes them to avoid an alias conflict (bevy-traps.md #3, \
              the dispatch_fire precedent); Bevy's IntoSystem inference rejects a \
              lifetime-generic type alias for the ParamSet, so it stays inline"
)]
/// Advance one step per frame for each walking ganger; stop on block, reveal, or interrupt.
pub fn advance_walk(
    mut world: bevy::ecs::system::ParamSet<(
        Query<(
            Entity,
            &'static mut Position,
            &'static mut Tu,
            &'static LifeState,
            &'static Faction,
            &'static mut WalkInProgress,
        )>,
        Query<(Entity, &'static Position, &'static Faction)>,
    )>,
    grid: Res<OccupancyGrid>,
    squad: Res<SquadVisibility>,
    mut reactions: MessageReader<ReactionShotFired>,
    mut moves: MessageWriter<MovementOccurred>,
    mut commands: Commands,
) {
    let interrupted: HashSet<Entity> = reactions.read().map(|shot| shot.mover).collect();

    let snapshot: Vec<(Entity, CellLevel, Faction)> = world
        .p1()
        .iter()
        .map(|(entity, position, faction)| (entity, **position, *faction))
        .collect();

    for (mover, mut position, mut tu, &life, &faction, mut walk) in &mut world.p0() {
        if !matches!(life, LifeState::Alive) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        if interrupted.contains(&mover) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        let visible_now = visible_enemy_cells(mover, faction, &snapshot, &squad);
        if *walk.reveals_new_enemy(&visible_now) {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        let Some((next, cost)) = walk.peek_next() else {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        };

        if *grid.is_path_blocked(&next) || grid.occupant(&next).is_some() {
            commands.entity(mover).remove::<WalkInProgress>();
            continue;
        }

        let from = position.cell();
        *position = Position::new(next);
        spend_tu(&mut tu, cost);
        moves.write(MovementOccurred::new(mover, from, next.cell()));
        walk.pop_next();

        if *walk.is_complete() {
            commands.entity(mover).remove::<WalkInProgress>();
        }
    }
}
