mod animate;
mod animation;
mod signal;

#[cfg(test)]
mod test;

pub use animate::{advance_impact_animations, seed_impact_animations};
pub(super) use animation::ImpactAnimation;
pub use signal::ShotImpactResolved;
