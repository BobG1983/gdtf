//! The input plugin (GTW-221 / GTW-225 / GTW-238 / GTW-259): [`GdtfBattleInputPlugin`] and
//! its [`GdtfBattleInputActive`] marker, which wire every battle input system into the
//! [`InputSystems::Gather`](crate::InputSystems) band ordered before the sim.

mod build;

#[cfg(test)]
mod test;

pub use build::{GdtfBattleInputActive, GdtfBattleInputPlugin};
