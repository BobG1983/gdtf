//! AC3 — `FireRequested` dispatch runs `fire()`: a shot at an in-line target mutates the
//! target's battle surfaces (real shot effect); same seed reproduces it. Plus GTW-242 —
//! the firing-arc + turn-to-fire gate layered on the fire dispatch. Value-agnostic on
//! every magnitude: the assertions are RELATIONS (the TU drop equals the fire cost in-arc,
//! the turn cost plus the fire cost out-of-arc, and is UNCHANGED on reject), never pinned.

mod support;

mod arc;
mod declaration;
mod dispatch;
mod report;
mod shot_fired;
