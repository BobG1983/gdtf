//! Muzzle position, aim point, and aim direction for a shot.

mod aim_dir;
mod aim_point;
mod muzzle;

#[cfg(test)]
mod test;

pub use aim_dir::{AimDir, climb_aim_dir};
pub use aim_point::target_aim_point;
pub use muzzle::muzzle_position;
pub(crate) use muzzle::{clamp_within_cell, muzzle_height};
