//! Fill-pass unit tests, split by concern: [`support`] holds the shared theme / prefab /
//! registry / tuning fixtures and coverage-accounting helpers; [`content`] pins what the fill
//! PRODUCES (GTW-427 C1 same-theme placement, C3 dead-space padding, and determinism under a
//! seed); [`termination`] pins what STOPS the fill (C2 nothing-fits + empty bucket, and the
//! GTW-767 max-coverage-cap termination). Wiring only: `mod` declarations, no test bodies.

mod content;
mod support;
mod termination;
