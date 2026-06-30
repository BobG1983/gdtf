//! Relocated unit tests for the `weapon` dir-module (GTW-201 code-health wave) — the
//! inline `#[cfg(test)] mod tests` moved VERBATIM into per-concern files, with the
//! shared fixtures in [`support`]. Wiring only: `mod` declarations, no test bodies.

mod support;

mod bundle;
mod components;
mod fire_mode;
mod melee;
mod spec_registry;
