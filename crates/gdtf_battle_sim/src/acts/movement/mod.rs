//! Move requests, walk state, and movement signals.

mod dispatch;
mod params;
mod signals;
mod suppression_gate;
mod walk;

pub use dispatch::dispatch_move;
pub use signals::{MoveRejected, MoveRejection, MovementOccurred};
pub use walk::{ReactionShotFired, RouteComplete, WalkInProgress, advance_walk};
