//! C5 (GTW-353, the user-ratified OQ-5 ruling) — the **visibility-gated planning**
//! tests: the search routes only through ROUTABLE cells (UNSEEN non-routable, EXPLORED
//! routable), and within routable cells applies the visibility-aware blocking
//! predicate (own-squad always blocks, an enemy blocks iff squad-VISIBLE, walls /
//! cover scatter always block).
//!
//! RELATIONS-ONLY, pin-DISCRIMINATING (each test flips a verdict by changing ONLY the
//! fog / occupant under test, never a magnitude): every fixture pairs the gated case
//! against a control so the visibility gate — not the geometry — is proven the cause.
//! All fog is hand-seeded; no shipped tunable magnitude is asserted.

mod support;

mod fog_routing;
mod link_relaxation;
mod occupant_blocking;
