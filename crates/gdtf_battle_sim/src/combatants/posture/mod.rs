//! Change stance, facing, and aiming (with TU cost).

mod cost;
#[cfg(test)]
mod test;
mod verbs;

pub use cost::{
    CanSetFacing, CanSetStance, FacingRefusal, StanceRefusal, afforded_turn_steps,
    afforded_turn_tu_cost, can_set_facing, can_set_stance, facing_refusal, set_aiming_tu_cost,
    stance_refusal, stance_tu_cost, turn_tu_cost,
};
pub use verbs::{FacingChanged, StanceChanged, set_aiming, set_facing, set_stance};
