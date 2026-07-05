//! The per-round composition helpers behind [`fire`](super::fire) — the shooter
//! `Copy`-snapshot ([`snapshot`]), the once-composed target geometry + the
//! constant-per-burst round setup + the per-round driver ([`round`]), the
//! primary-impact fold ([`fold`]), and the GTW-541 `AoE` splash pass
//! ([`splash`]) — the verbs ([`read_shooter`] / [`resolve_round`]) that read the
//! shooter off its query and resolve one round of the burst.
//!
//! These are fire-private composition steps (`pub(in crate::shot_pipeline::fire)`
//! for [`volley`](super::fire) to call); the public surface is the query shapes
//! ([`query`](super::query)) and [`fire`](super::fire) itself.

mod fold;
mod round;
mod snapshot;
mod splash;

pub(in crate::shot_pipeline::fire) use round::{RoundSetup, TargetGeometry, resolve_round};
pub(in crate::shot_pipeline::fire) use snapshot::{ShooterReads, read_shooter};
