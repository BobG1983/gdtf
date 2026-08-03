//! Change stance, facing, and aiming (with TU cost).

#[cfg(test)]
mod test;
mod verbs;

pub use verbs::{FacingChanged, StanceChanged, set_aiming, set_facing, set_stance};
