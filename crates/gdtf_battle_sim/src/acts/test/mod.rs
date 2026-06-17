//! Relocated unit tests for the `acts` dispatch dir-module (GTW-201 code-health wave) —
//! the inline `#[cfg(test)] mod tests` moved VERBATIM into per-concern files, with the
//! shared fixtures in [`support`]. Wiring only: `mod` declarations, no test bodies here.

mod support;

mod coschedule;
mod downed;
mod fire;
mod movement;
mod plugin;
mod posture;
mod request;
