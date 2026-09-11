//! Enemy turn AI: snapshot, target pick, downed acts, advance, engage, reload, melee, shove,
//! doors, aim, crouch.

mod advance;
mod brain;
mod decide;
mod door;
mod downed;
mod engage;
mod params;
mod posture;
mod shove;
mod snapshot;

#[cfg(test)]
mod test;

pub use brain::enemy_ai_turn;
pub use decide::{AiTarget, pick_nearest, plan_advance};
