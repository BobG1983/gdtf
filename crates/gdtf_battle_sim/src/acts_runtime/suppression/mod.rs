//! The **suppression core** — the ECS runtime that pins a ganger under enemy fire
//! (GTW-526, child of GTW-41; `docs/combat/combat.md` "Suppression … advanced combat
//! effects").
//!
//! A ganger fired near by an OPPONENT is [`Suppressed`](crate::ganger::Suppressed): a
//! worse reactor (it cannot reaction-fire while pinned) that reflexively drops behind its
//! nearest cover. Symmetric — BOTH factions can be suppressed. The state is a per-ganger
//! component carrying the [`SuppressorCell`](crate::ganger::SuppressorCell) the fire came
//! from; this module owns the three systems that maintain it. The reaction-fire lockout
//! (C3) lives in [`reaction_trigger`](crate::reaction::reaction_trigger) (the eligibility
//! gate skips a suppressed reactor BEFORE any [`ReactionRng`](crate::rng::ReactionRng)
//! draw, so determinism is unperturbed).
//!
//! ## Module map
//!
//! - `apply` — the PRODUCER [`apply_suppression`]: read enemy
//!   [`FireRequested`](crate::acts::FireRequested), mark every OPPOSING ganger within the
//!   tuning [`SuppressionRadius`](crate::tuning::SuppressionRadius) of the shot's target
//!   as [`Suppressed`](crate::ganger::Suppressed), and emit a [`SuppressionApplied`] FCT
//!   signal on a FRESH application (an idempotent refresh emits nothing, no stacking).
//! - `stance` — the AUTO-STANCE drop [`suppression_auto_stance`] (on
//!   [`Added<Suppressed>`](bevy::prelude::Added)) + the pure band→posture mapping
//!   [`stance_for_cover_band`]: duck the ganger behind the cover one step toward the
//!   suppressor, writing [`Stance`](crate::ganger::Stance) DIRECTLY (no TU).
//! - `reset` — the CLEAR cadence [`reset_suppression`]: on a
//!   [`TurnStarted`](crate::turn::TurnStarted), remove
//!   [`Suppressed`](crate::ganger::Suppressed) from every ganger of the now-active faction
//!   (faction-scoped, so a unit stays pinned through the opponent's turn and clears at its
//!   OWN turn-start).
//!
//! The schedule WIRING lives in [`SimActsPlugin`](crate::acts::SimActsPlugin). Param-only
//! throughout — `Query` / `Res` / `Commands` / `MessageReader` / `MessageWriter`, no
//! `&mut World` (`bevy-traps.md` #7).

mod apply;
mod reset;
mod stance;

#[cfg(test)]
mod test;

pub use apply::{SuppressionApplied, apply_suppression};
pub use reset::reset_suppression;
pub use stance::{stance_for_cover_band, suppression_auto_stance};
