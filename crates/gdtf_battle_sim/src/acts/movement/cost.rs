//! TU cost and legality queries for a planned walk.

use bevy::prelude::Deref;

use crate::{ganger::Tu, pathfinder::Path, tu::can_spend_tu};

/// Whether the mover can pay for a planned route.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanMove(bool);

impl CanMove {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
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

/// The pool covers the whole route.
#[must_use]
pub fn can_move(tu: &Tu, path: &Path) -> CanMove {
    CanMove::new(*can_spend_tu(tu, move_tu_cost(path)))
}
