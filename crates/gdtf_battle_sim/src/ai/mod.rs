//! Enemy turn AI: snapshot, target pick, advance, engage, reload, melee.

mod advance;
mod brain;
mod decide;
mod engage;
mod params;
mod snapshot;

#[cfg(test)]
mod test;

pub use brain::enemy_ai_turn;
pub use decide::{AiTarget, pick_nearest, plan_advance};
