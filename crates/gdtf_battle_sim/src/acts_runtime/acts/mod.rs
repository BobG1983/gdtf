//! The message-driven **input contract** for the landed combat acts + the per-act
//! **dispatch systems** that consume it + the public [`SimActsPlugin`] that registers
//! both — the SIM SIDE of E10's headless boundary (E10.2 / GTW-204).
//!
//! E10 drives the authoritative sim from BUFFERED Bevy MESSAGES, not direct calls: a
//! later CP437-input slice (out of scope here) emits a `*Requested` message per player
//! act, and ONE dispatch system per act drains its
//! [`MessageReader`](bevy::prelude::MessageReader), fetches the actor/target components
//! via Bevy queries, and calls the ALREADY-LANDED verb once per message. No act logic is
//! reimplemented here — every dispatch system REUSES the landed verb
//! ([`fire`](crate::fire::fire) / [`set_aiming`](crate::posture::set_aiming) /
//! [`set_stance`](crate::posture::set_stance) / [`set_facing`](crate::posture::set_facing)
//! / [`stabilize_downed`](crate::downed_acts::stabilize_downed) /
//! [`execute_downed`](crate::downed_acts::execute_downed)) and its query/bundle shapes
//! verbatim.
//!
//! ## Module map
//!
//! - [`request`] — the eight [`#[derive(Message)]`](bevy::prelude::Message) `*Requested`
//!   types (the input contract; the eighth, [`ReloadRequested`], added in GTW-275) +
//!   the [`AimRequest`] aim-flag newtype. Each carries the
//!   act's [`Entity`](bevy::prelude::Entity) actor ref(s) plus the act's OWNED payload; a
//!   `Message` cannot hold a borrow, so [`FireRequested`] carries an OWNED
//!   [`FireModeSpec`](crate::weapon::FireModeSpec) (now `Copy` again, GTW-260) and has
//!   **no lifetime parameter**.
//! - [`fire`] — the [`dispatch_fire`] system with the GTW-242 firing-arc + turn-to-fire
//!   gate, the [`BattleGridsParam`] system-param bundle, and the pure pre-mutation arc
//!   decision. The turn-write + [`fire`](crate::fire::fire) re-borrow are time-multiplexed
//!   through a [`ParamSet`](bevy::ecs::system::ParamSet) (`bevy-traps.md` #3 / #7).
//! - [`posture`] — the [`dispatch_set_aiming`] / [`dispatch_set_stance`] /
//!   [`dispatch_set_facing`] systems (E10.2 AC4).
//! - [`downed`] — the [`dispatch_stabilize_downed`] / [`dispatch_execute_downed`] systems
//!   (E10.2 AC5).
//! - [`movement`] — the [`dispatch_move`] system (E4 / GTW-234).
//! - [`reload`] — the [`dispatch_reload`] system (GTW-275): the real, TU-costed
//!   reload act, charging the actor's own per-weapon
//!   [`Magazine::reload_tu`](crate::magazine::Magazine::reload_tu) and refilling the
//!   magazine to full.
//! - [`plugin`] — the public [`SimActsPlugin`] registration unit: it
//!   [`add_message`](bevy::app::App::add_message)s all eight types exactly once each
//!   (`bevy-traps.md` #5) and adds the eight dispatch systems
//!   `.in_set(SimSystems::Simulate)` in [`Update`](bevy::prelude::Update) — consuming
//!   E10.0's set (it imports and uses it, never redefines it, and never calls
//!   `configure_sets`, which is E10.0's). So the dispatch systems compose deterministically
//!   with the
//!   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
//!   systems already in that set.
//!
//! Render-free, deterministic model logic: every random draw bottoms out in the single
//! injected [`ShotRng`](crate::rng::ShotRng)/[`SeverityRng`](crate::rng::SeverityRng) resource (the fire dispatch system's
//! `ResMut<ShotRng> + ResMut<SeverityRng>`); no renderer, no window, no presenter, no pixel.

mod downed;
mod fire;
mod injury;
mod melee;
mod movement;
mod plugin;
mod posture;
mod reload;
mod request;

#[cfg(test)]
mod test;

pub use downed::{dispatch_execute_downed, dispatch_stabilize_downed};
pub use fire::{
    BattleGridsParam, FireArcDecision, FireDeclaration, can_engage, decide_fire_arc, dispatch_fire,
};
pub use injury::{InjuryInflicted, apply_injury};
pub use melee::dispatch_melee;
pub use movement::{MoveRejected, MoveRejection, MovementOccurred, dispatch_move};
pub use plugin::SimActsPlugin;
pub use posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance};
pub use reload::{ReloadOutcome, ReloadResult, dispatch_reload};
pub use request::{
    AimRequest, EndTurnRequested, ExecuteDownedRequested, FireRequested, MeleeRequested,
    MeleeResolved, MeleeTarget, MoveRequested, ReloadRequested, SetAimingRequested,
    SetFacingRequested, SetStanceRequested, StabilizeDownedRequested,
};
