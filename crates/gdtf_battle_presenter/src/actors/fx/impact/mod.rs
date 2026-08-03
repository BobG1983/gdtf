mod animate;
mod animation;
mod signal;

#[cfg(test)]
mod test;

pub use animate::animate_impact;
pub(super) use animation::ImpactAnimation;
pub use signal::ShotImpactResolved;
