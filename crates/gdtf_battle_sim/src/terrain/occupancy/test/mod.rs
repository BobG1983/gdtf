//! Behavior-preserving split of the occupancy-grid module tests (GTW-583) — one
//! file per concern, with shared grid fixtures + neighbour probes in [`support`].

mod support;

mod blocking;
mod build;
mod cost_factor;
mod neighbours;
