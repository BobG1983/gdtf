//! Apply a resolved hit to a living combatant.

mod fold;

pub use fold::{GangerHitTarget, apply_hit};

#[cfg(test)]
mod test;
