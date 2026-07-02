//! The ECS message-driven act/turn runtime: acts dispatch, movement, downed verbs, turn cycle, bleed, damage-over-time, area-damage fields, on-death effects, weapon-attachment application, firing arc, minimal enemy AI, reaction fire, suppression.

pub mod acts;
pub mod ai;
pub mod attachments;
pub mod bleed;
pub mod dot;
pub mod downed_acts;
pub mod fields;
pub mod firing_arc;
pub mod move_acts;
pub mod on_death;
pub mod reaction;
pub mod suppression;
pub mod turn;
