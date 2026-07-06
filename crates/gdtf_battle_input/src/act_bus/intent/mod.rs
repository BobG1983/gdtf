//! The shared ACT-INTENT data seam (GTW-225 / GTW-48 S8 AC9): the ONE buffered
//! intent surface BOTH input surfaces write — the 222b keyboard systems AND the
//! 222c `gdtf_app` action-bar buttons — drained by ONE dispatch system.
//!
//! # Why a DATA seam, not a shared `fn`
//!
//! "Buttons + keys are parallel surfaces over the SAME act dispatch" must be REAL
//! across the crate boundary. A `SystemParam`-taking dispatch system in
//! `gdtf_battle_input` cannot be CALLED by a `bevy_ui` button system in `gdtf_app`
//! (you cannot invoke one Bevy system from inside another). The only thing that
//! spans the one legal `gdtf_app -> gdtf_battle_input` edge is DATA: both surfaces
//! [`push`](PendingActIntent::push) an [`ActIntent`] into the [`PendingActIntent`]
//! queue, and [`dispatch_act_intents`] drains it. ADR-0001's
//! `input -> presenter -> sim` chain stays acyclic — `gdtf_app` depends on
//! `gdtf_battle_input`, never the reverse.
//!
//! # What 222a owns vs. what 222b (GTW-227) fills
//!
//! 222a owns the no-act intents: [`ActIntent::SelectionClear`] /
//! [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] — drained here directly
//! (clearing [`SelectedShooter`](crate::SelectedShooter) / clamping
//! [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel)). The act-bearing variants
//! ([`ActIntent::StanceCycle`] etc.) were DECLARED there so the seam's shape
//! is fixed from the start; 222b (GTW-227) FILLS their drain arms — emitting the
//! matching `gdtf_battle_sim::acts::*Requested` for the
//! [`SelectedShooter`](crate::SelectedShooter), reading the actor's CURRENT
//! [`Stance`](gdtf_battle_sim::ganger::Stance) / [`Facing`](gdtf_battle_sim::ganger::Facing) /
//! [`Aiming`](gdtf_battle_sim::ganger::Aiming) off a query and stepping the authored
//! [`crate::cycle`] order — and adds the [`ActIntent::Fire`] variant the left-click
//! FIRE surface writes (its `can_fire` guard runs at the WRITE site, so the drain
//! just emits the carried [`FireRequested`](gdtf_battle_sim::acts::FireRequested)
//! payload).

mod level;
mod seam;

#[cfg(test)]
mod test;

pub use level::{LevelStep, step_level};
pub use seam::{
    ActIntent, ActWriters, PendingActIntent, SelectionCycleReads, dispatch_act_intents,
};
