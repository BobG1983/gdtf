//! Time-unit spend and reset helpers used across the sim.

mod economy;
#[cfg(test)]
mod test;

pub use economy::{TuAffordable, can_spend_tu, reset_tu, spend_tu};
