//! The ECS message-driven act/turn runtime: acts dispatch, movement, downed verbs, turn cycle, bleed, firing arc, minimal enemy AI.

pub mod acts;
pub mod ai;
pub mod bleed;
pub mod downed_acts;
pub mod firing_arc;
pub mod move_acts;
pub mod turn;
