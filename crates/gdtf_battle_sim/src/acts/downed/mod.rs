//! The §9 from-Downed verbs + their shared faction-aware predicates (the E3.8
//! slice, GTW-190) + the E10.2 dispatch systems that drive them — one home for
//! the downed acts (GTW-638 re-homed the verbs from the dissolved
//! `acts_runtime::downed_acts`).
//!
//! Two adjacency verbs act on a [`crate::ganger::LifeState::Downed`] ganger, and
//! each shares its guard with a predicate the HUD button and the act both call (one
//! guard set, so the button is enabled **exactly** when the act would succeed —
//! `docs/combat/resolution.md` §9: "enabled off the model's own `can_stabilize`
//! predicate so button and act share one guard set"):
//!
//! - [`stabilize_downed`] — an 8-adjacent ALIVE **ally** dresses the wound, halting
//!   the bleed-out clock: it **removes** the [`crate::effects::bleed::BleedingOut`]
//!   condition (GTW-695 — the §9 clock is reified as a removable marker, not a negation
//!   flag; [`crate::effects::bleed::tick_bleed`] *reads* its presence). The ganger
//!   **remains [`crate::ganger::LifeState::Downed`]** — NOT Alive, NOT Dead.
//! - [`execute_downed`] — an 8-adjacent ALIVE **enemy** finishes the Downed ganger
//!   outright → [`crate::ganger::LifeState::Dead`].
//!
//! ## The faction differentiator
//!
//! `Faction` is wired INTO both predicates — it is what tells an ally apart from an
//! enemy (`docs/combat/wounds-and-roster.md` §"Downed → death … state machine":
//! stabilize is an **ally** action, execute an **enemy** one):
//!
//! - [`can_stabilize`] requires `actor.faction == target.faction` (an **ally**) — an
//!   enemy can **never** stabilize.
//! - [`can_execute`] requires `actor.faction != target.faction` (an **enemy**) — an
//!   ally can **never** execute.
//!
//! ## 8-adjacency (same-level Moore-8)
//!
//! [`is_8_adjacent`] is the literal "8 surrounding cells": the same
//! [`crate::metric::Level`] (`z` equal) **and** Chebyshev distance 1 in the `x`/`y`
//! cell plane (`max(|dx|, |dy|) == 1`), excluding the same cell. An actor a storey
//! above or below is **not** adjacent — reach is the 8 same-storey neighbours, not
//! the 26 cross-level voxels (`docs/combat/resolution.md` §9: an "8-adjacent"
//! actor).
//!
//! ## TU boundary (E4)
//!
//! The flat [`crate::tuning::StabilizeTu`] / [`crate::tuning::ExecuteTu`] costs are
//! **READ** from tuning to wire the leaf (each act returns the cost it consulted),
//! but this slice runs **no TU economy**: it never debits a [`crate::ganger::Tu`]
//! pool nor checks can-afford — that is **E4**. Pure, render-free model logic: the
//! predicates operate on component values, the acts mutate component references; no
//! renderer, no pixel.
//!
//! The `dispatch` submodule holds the E10.2 AC5 message-driven boundary: the
//! [`dispatch_stabilize_downed`] / [`dispatch_execute_downed`] systems drain each
//! buffered `*Requested` message and run the landed verb once per message.

mod dispatch;
mod execute;
mod reach;
mod stabilize;

#[cfg(test)]
mod test;

pub use dispatch::{dispatch_execute_downed, dispatch_stabilize_downed};
pub use execute::{CanExecute, can_execute, execute_downed};
pub use reach::{Actor, Adjacent8, DownedTarget, is_8_adjacent};
pub use stabilize::{CanStabilize, can_stabilize, stabilize_downed};
