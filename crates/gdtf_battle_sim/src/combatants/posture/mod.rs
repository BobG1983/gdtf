//! Change stance, facing, and aiming (with TU cost).

mod cost;
#[cfg(test)]
mod test;
mod verbs;

pub use cost::{
    CanSetFacing, CanSetStance, afforded_turn_steps, afforded_turn_tu_cost, can_set_facing,
    can_set_stance, stance_tu_cost, turn_tu_cost,
};
pub use verbs::{FacingChanged, StanceChanged, set_aiming, set_facing, set_stance};
