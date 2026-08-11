//! TU cost and legality queries for a planned walk.

use bevy::prelude::Entity;

use super::{
    sight::SightWorld,
    suppression_gate::{BreakAwayMover, suppressed_move_legal},
};
use crate::{
    cover::CoverLedger,
    ganger::{Facing, Position, Stance, Suppressed, Tu},
    metric::CellLevel,
    pathfinder::Path,
    tu::can_spend_tu,
};

/// Whether a planned walk is allowed, and what turns it down when it is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveVerdict {
    /// Nothing refuses the walk.
    Allowed,
    /// Suppression holds the mover and the route does not break away from it.
    Suppressed,
    /// The pool does not cover the route.
    Unaffordable,
}

/// The mover a walk is judged for: who it is, where it stands and how, what it can spend, what
/// pins it.
#[derive(Debug, Clone, Copy)]
pub struct Mover<'a> {
    entity:      Entity,
    at:          &'a Position,
    tu:          &'a Tu,
    stance:      &'a Stance,
    facing:      &'a Facing,
    suppression: Option<&'a Suppressed>,
}

impl<'a> Mover<'a> {
    /// Build from the mover, its cell and pose, its pool, and its suppression marker if it has one.
    #[must_use]
    pub const fn new(
        entity: Entity,
        at: &'a Position,
        tu: &'a Tu,
        stance: &'a Stance,
        facing: &'a Facing,
        suppression: Option<&'a Suppressed>,
    ) -> Self {
        Self {
            entity,
            at,
            tu,
            stance,
            facing,
            suppression,
        }
    }

    // What the break-away gate reads off the mover.
    const fn break_away(&self) -> BreakAwayMover<'a> {
        BreakAwayMover::new(self.entity, self.at, self.stance, self.facing)
    }
}

/// TU charged for walking a whole route.
#[must_use]
pub const fn move_tu_cost(path: &Path) -> Tu {
    path.total()
}

/// TU charged for each step of the route, in walk order.
#[must_use]
pub fn move_step_tu_costs(path: &Path) -> &[Tu] {
    path.steps()
}

/// Whether this mover may walk this route to `dest`.
/// Suppression is judged first, then the pool.
#[must_use]
pub fn can_move<F: Fn(Entity) -> bool>(
    mover: Mover<'_>,
    dest: &CellLevel,
    path: &Path,
    cover: &CoverLedger,
    sight: &SightWorld<'_, F>,
) -> MoveVerdict {
    let breaks_away = mover.suppression.is_none_or(|suppressed| {
        *suppressed_move_legal(&mover.break_away(), dest, &suppressed.from, cover, sight)
    });
    if !breaks_away {
        return MoveVerdict::Suppressed;
    }
    if !*can_spend_tu(mover.tu, move_tu_cost(path)) {
        return MoveVerdict::Unaffordable;
    }
    MoveVerdict::Allowed
}
