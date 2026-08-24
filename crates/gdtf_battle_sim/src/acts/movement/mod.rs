//! Move requests, walk state, and movement signals.

mod cost;
mod dispatch;
mod mount;
mod params;
mod sight;
mod signals;
mod suppression_gate;
mod walk;

pub use cost::{MoveVerdict, Mover, can_move, move_step_tu_costs, move_tu_cost};
pub use dispatch::dispatch_move;
pub use mount::{DismountSurcharge, dismount_surcharge, seat_departure, seat_surcharge};
pub use sight::SightWorld;
pub use signals::{MoveCompleted, MoveRejected, MoveRejection, MovementOccurred};
pub(crate) use suppression_gate::{BreakAwayMover, suppressed_move_legal};
pub use walk::{
    ReactionShotFired, RouteComplete, WalkInProgress, WalkSignals, WalkStepped, advance_walk,
};
