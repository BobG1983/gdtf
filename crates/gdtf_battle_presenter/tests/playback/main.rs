//! GTW-727 — the PRESENTER's playback cursor: the mechanism that lets the view show a
//! sim tick's worth of acts one at a time, without the sim ever waiting.
//!
//! - **`pacing`** — the readable-pacing proof (T9): five reaction shots the sim resolved in
//!   one tick reach the screen one at a time, each with its own beat. Plus the two impact
//!   traps (T10 pre-spawn gap, T11 the no-bolt backstop) and the soft-lock regression guard
//!   (T12: the catch-up predicate reads no entity population).
//! - **`drawn_lag`** — the tests this whole ticket turns on (T13 / T14): the DRAWN position
//!   and the DRAWN pose lag the sim until their entry is played. A paced combat log with
//!   snapping sprites is the reported symptom shipped as fixed; these are what make that
//!   impossible. Plus T15, the gap recovery.
//! - **`cursor_time_fog`** — GTW-762: the squad-fog SHADOW (`ShownSquadVisibility`) freezes
//!   during closed-gate playback and promotes the instant the cursor catches up, so the
//!   terrain fog reads the cursor's playback position, not the sim's live (ahead) state.
//!
//! NO WALL-CLOCK WAITS. Every test advances `Time` by an exact duration and runs the cursor
//! once, so a pacing assertion is deterministic by construction.
//!
//! Wiring only.

mod cursor_time_fog;
mod drawn_lag;
mod harness;
mod pacing;
