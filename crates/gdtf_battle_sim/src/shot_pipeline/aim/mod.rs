//! Build the aim cone and stability multipliers for a shooter.

mod compose;
mod shooter;

#[cfg(test)]
mod test;

pub use compose::{cone_for, stability_for};
pub use shooter::Shooter;
