//! Move requests, walk state, and movement signals.

mod cost;
mod dispatch;
mod params;
mod signals;
mod suppression_gate;
mod walk;

pub use cost::{CanMove, can_move, move_step_tu_costs, move_tu_cost};
pub use dispatch::dispatch_move;
pub use signals::{MoveRejected, MoveRejection, MovementOccurred};
pub(crate) use suppression_gate::suppressed_move_legal;
pub use walk::{ReactionShotFired, RouteComplete, WalkInProgress, advance_walk};
