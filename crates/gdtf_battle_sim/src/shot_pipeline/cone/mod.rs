//! Shot cone angle and the factors that widen or tighten it.

mod angle;
mod factors;

#[cfg(test)]
mod test;

pub use angle::{ConeAngle, cone_angle};
pub use factors::{PriorShots, RecoilFactor, aim_cone_mult, recoil_factor};
