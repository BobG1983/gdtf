//! Tests for the minimal enemy AI (GTW-70 / GTW-461) — the PURE decision functions
//! ([`decide`](super::decide)), the END-TO-END brain integration through the real systems
//! ([`brain`](super::brain)), and the GTW-461 act-cadence pacing ([`cadence`](super::cadence)
//! exercised end-to-end in [`cadence`]). Shared fixtures live in [`support`]. Wiring only:
//! `mod` declarations, no test bodies here.

mod support;

mod brain;
mod cadence;
mod decide;
