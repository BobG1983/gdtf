//! The act-log RECORDER — the one registered writer and its six per-family recorders
//! (GTW-727 C9).
//!
//! `pass` holds [`record_acts`], the ONE system that appends; `sources` holds the
//! [`SystemParam`](bevy::ecs::system::SystemParam) reader bundles; the remaining leaves
//! are one act family each, invoked in a fixed source order that IS the log's intra-tick
//! ordering guarantee.
//!
//! Wiring only.

mod consequence;
mod fire;
mod life;
mod movement;
mod pass;
mod posture;
mod sources;
mod turn;

pub use pass::record_acts;
