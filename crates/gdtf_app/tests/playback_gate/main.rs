//! GTW-727 clause (a) — the GLOBAL INPUT GATE: while the presenter is still showing what
//! already happened, the player cannot act on it.
//!
//! - **`discard`** — the drain-and-discard ruling (T16): the intent queue is drained every
//!   frame even while the gate is shut, and act-bearing intents are DROPPED rather than
//!   held. Deferring them would be worse than dropping them: the queue is a `mem::take`, so
//!   a skipped drain accumulates a backlog that fires in one burst the instant the gate
//!   opens. Plus T17, the positive half: the view controls stay live, so an over-broad gate
//!   regresses loudly.
//! - **`fail_open`** — clause (d) (T23): an app with no presenter dispatches acts exactly as
//!   it always did. This is the guard on the single most dangerous line in the change.
//!
//! The QA-net half (T21 / T22) lives in the `net_qa` suite, which carries the feature gate
//! that whole surface compiles under: `net_qa/caught_up.rs`.
//!
//! Wiring only.

mod discard;
mod fail_open;
mod support;
