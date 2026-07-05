//! Behavior-preserving split of the stability module tests (GTW-583) — one file
//! per concern, with the shared cover fixture in [`support`].

mod support;

mod brace_parity;
mod curve;
mod gate;
mod score;
mod types;
