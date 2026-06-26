//! Relocated unit tests for the `fire` volley dir-module (GTW-201 code-health wave) —
//! the inline `#[cfg(test)] mod tests` moved VERBATIM into per-AC concern files, with
//! the shared fixtures in [`support`]. Wiring only: `mod` declarations, no test bodies
//! here.

mod support;

mod access;
mod application;
mod determinism;
mod economy;
mod fail_closed;
mod handedness;
mod recoil;
