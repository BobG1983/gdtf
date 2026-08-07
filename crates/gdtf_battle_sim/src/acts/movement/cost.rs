//! TU cost and legality queries for a planned walk.

use super::suppression_gate::suppressed_move_legal;
use crate::{
    cover::CoverLedger,
    ganger::{Position, Suppressed, Tu},
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

/// The mover a walk is judged for: where it stands, what it can spend, what suppresses it.
#[derive(Debug, Clone, Copy)]
pub struct Mover<'a> {
    at:          &'a Position,
    tu:          &'a Tu,
    suppression: Option<&'a Suppressed>,
}

impl<'a> Mover<'a> {
    /// Build from the mover's cell, its pool, and its suppression marker when it carries one.
    #[must_use]
    pub const fn new(at: &'a Position, tu: &'a Tu, suppression: Option<&'a Suppressed>) -> Self {
        Self {
            at,
            tu,
            suppression,
        }
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
pub fn can_move(
    mover: Mover<'_>,
    dest: &CellLevel,
    path: &Path,
    cover: &CoverLedger,
) -> MoveVerdict {
    let breaks_away = mover
        .suppression
        .is_none_or(|suppressed| *suppressed_move_legal(mover.at, dest, &suppressed.from, cover));
    if !breaks_away {
        return MoveVerdict::Suppressed;
    }
    if !*can_spend_tu(mover.tu, move_tu_cost(path)) {
        return MoveVerdict::Unaffordable;
    }
    MoveVerdict::Allowed
}
