//! Behavior-preserving split of the occupancy-sync module tests (GTW-583) — one
//! file per maintenance concern, with the shared headless harness + grid probes
//! in [`support`].

mod support;

mod band;
mod cover;
mod life_state;
mod movement;
mod schedule;
mod stair_presence;
mod stair_transitions;
