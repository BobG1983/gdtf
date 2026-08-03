//! Enemy turn AI: snapshot, target pick, advance, engage.

mod advance;
mod brain;
mod decide;
mod engage;
mod snapshot;

#[cfg(test)]
mod test;

pub use brain::enemy_ai_turn;
pub use decide::{AiTarget, pick_nearest, plan_advance};
