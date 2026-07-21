//! The **minimal enemy AI** (GTW-70) — a render-free, deterministic-without-RNG enemy turn
//! brain that makes each enemy ganger engage-or-advance-or-hold over the EXISTING combat
//! surfaces, so the situation is playable end-to-end against an opponent that acts.
//!
//! The model never reimplements combat: the brain emits the REAL `*Requested` messages the
//! landed dispatchers resolve (a [`FireRequested`](crate::acts::FireRequested) →
//! [`dispatch_fire`](crate::acts::dispatch_fire) → `fire()`, a
//! [`MoveRequested`](crate::acts::MoveRequested) → faction-aware
//! [`dispatch_move`](crate::acts::dispatch_move) → `find_path`, an
//! [`EndTurnRequested`](crate::acts::EndTurnRequested) → the turn cycle). It shares ONE
//! engagement check with the fire dispatcher (the GTW-70 leaf-2
//! [`can_engage`](crate::acts::can_engage) arc verdict) and ONE move fog with the move
//! dispatcher (the GTW-70 leaf-3 [`OmniscientFog`](crate::visibility::OmniscientFog) +
//! `move_fog` selector), so planner and executor can never disagree.
//!
//! ## Module map
//!
//! - `decide` — the PURE decision functions (ECS-free, unit-testable): [`AiTarget`],
//!   [`pick_nearest`] (the §C nearest-target / advance-goal pick), and [`plan_advance`]
//!   (the §D.2 reposition step). No RNG, no `HashMap` / `Entity`-id order — a `(level, y,
//!   x)` total order at every choice point.
//! - `brain` — the [`enemy_ai_turn`] ECS system that, on the enemy faction's turn, runs
//!   the engage/advance/hold pass and ends the turn back to the player when done. Param-only
//!   (no `&mut World`, `bevy-traps.md` #7); wired into the sim by
//!   [`SimActsPlugin`](crate::acts::SimActsPlugin).
//!
//! **Pacing is NOT the brain's job** (GTW-727). The brain used to carry a tick-count act
//! cadence that made it wait between enemy acts, so that each act's world mutation landed
//! on its own frame and every `Changed<T>` view paced for free. That coupled how FAST the
//! sim resolved a turn to how READABLE the turn looked, which could only ever pace the one
//! act family it gated and never the several facts a single act produces. The presenter now
//! owns pacing outright, replaying the sim's act log at its own speed, so the brain runs at
//! full speed and holds no pacing state at all.
//!
//! **Scope (minimal, GTW-70 §F).** Cover-seeking, target *scoring* (lowest-HP /
//! best-hit-chance), a symmetric enemy fog-of-war (planning on last-seen positions, not the
//! omniscient move fog), multi-mode tactics, and AI-decision visualization are all DEFERRED
//! to GTW-71 / GTW-84 — flagged, never silently dropped.

mod advance;
mod brain;
mod decide;
mod engage;
mod snapshot;

#[cfg(test)]
mod test;

pub use brain::enemy_ai_turn;
pub use decide::{AiTarget, pick_nearest, plan_advance};
