//! Tests for the minimal enemy AI (GTW-70) — the PURE decision functions
//! ([`decide`](super::decide)) and the END-TO-END brain integration through the real
//! systems ([`brain`](super::brain)). Shared fixtures live in [`support`]. Wiring only:
//! `mod` declarations, no test bodies here.

mod support;

mod brain;
mod decide;
