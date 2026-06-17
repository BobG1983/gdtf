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
//! - [`request`] — the seven [`#[derive(Message)]`](bevy::prelude::Message) `*Requested`
//!   types (the input contract) + the [`AimRequest`] aim-flag newtype. Each carries the
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
//! - [`plugin`] — the public [`SimActsPlugin`] registration unit: it
//!   [`add_message`](bevy::app::App::add_message)s all seven types exactly once each
//!   (`bevy-traps.md` #5) and adds the seven dispatch systems
//!   `.in_set(SimSystems::Simulate)` in [`Update`](bevy::prelude::Update) — consuming
//!   E10.0's set (it imports and uses it, never redefines it, and never calls
//!   `configure_sets`, which is E10.0's). So the dispatch systems compose deterministically
//!   with the
//!   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
//!   systems already in that set.
//!
//! Render-free, deterministic model logic: every random draw bottoms out in the single
//! injected [`SimRng`](crate::rng::SimRng) resource (the fire dispatch system's
//! `ResMut<SimRng>`); no renderer, no window, no presenter, no pixel.

mod downed;
mod fire;
mod movement;
mod plugin;
mod posture;
mod request;

#[cfg(test)]
mod test;

pub use downed::{dispatch_execute_downed, dispatch_stabilize_downed};
pub use fire::{BattleGridsParam, dispatch_fire};
pub use movement::dispatch_move;
pub use plugin::SimActsPlugin;
pub use posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance};
pub use request::{
    AimRequest, ExecuteDownedRequested, FireRequested, MoveRequested, SetAimingRequested,
    SetFacingRequested, SetStanceRequested, StabilizeDownedRequested,
};
