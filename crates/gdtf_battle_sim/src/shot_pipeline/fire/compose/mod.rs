mod fold;
mod round;
mod snapshot;
mod splash;

pub(in crate::shot_pipeline::fire) use round::{RoundSetup, TargetGeometry, resolve_round};
pub(in crate::shot_pipeline::fire) use snapshot::{ShooterReads, read_shooter};
