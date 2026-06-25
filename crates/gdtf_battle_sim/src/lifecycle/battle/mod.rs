//! The sim-owned **battle-lifecycle integration**: the public [`BattleSimPlugin`]
//! that wires the whole render-free combat runtime into a Bevy
//! [`App`](bevy::prelude::App), plus the three message-driven lifecycle types the app
//! drives it with (E10.5 / GTW-207).
//!
//! This module is the model's OWN integration seam
//! (`docs/decisions/0001-rust-bevy-rewrite.md`: the model is the authoritative
//! render-free sim, consumed ONE-WAY by the app). Because the dependency edge is
//! `gdtf_app -> gdtf_battle_sim` (E10.1), the sim CANNOT and MUST NOT name any
//! `gdtf_app` type — no `AppState` / `BattleScapeState` / `GenerationComplete` /
//! `LoadedSituation`. So the battle lifecycle is decoupled from the app's state
//! machine via buffered [`Message`](bevy::prelude::Message)s: the app SENDS triggers
//! (naming its own app states, app-side) and the sim ACTS on them (naming only sim
//! types). This matches the message-driven canon E10.2 established (the app emits a
//! `*Requested`, the sim listens — `bevy-traps.md` #4: buffered
//! [`Message`](bevy::prelude::Message), NOT the observer `Event`).
//!
//! ## The three lifecycle messages
//!
//! - [`SetupBattleRequested`] — the SETUP trigger: an OWNED
//!   [`Situation`](crate::situation::Situation) (it is [`Clone`]) + a
//!   [`BattleSeed`](crate::rng::BattleSeed) (a [`Copy`] newtype). No lifetime, no app
//!   type.
//! - [`TeardownBattleRequested`] — the TEARDOWN trigger.
//! - [`BattleReady`] — the setup-complete SIGNAL the app gates its state advance on.
//!
//! ## [`BattleSimPlugin`]
//!
//! Adding this ONE plugin wires the whole sim runtime: it bundles
//! [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
//! (the E1.7 maintenance layer + the
//! [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) buffer) and the E10.2
//! [`SimActsPlugin`](crate::acts::SimActsPlugin) (the per-act dispatch + the six
//! `*Requested` buffers), registers the three lifecycle messages once each, and adds a
//! setup + a teardown system in [`Update`](bevy::prelude::Update), ordered around the
//! [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band (E10.0's
//! set).
//!
//! Because the bundled dispatch / occupancy systems read the battle-lifetime resources
//! `unconditionally` (a Bevy `Res<T>` whose resource is absent fails param validation —
//! `bevy-traps.md` #1), and those resources exist only DURING a battle, the plugin
//! gates the whole `Simulate` band on the purpose-built [`BattleInProgress`] witness so
//! the bundled runtime stays inert (and panic-free) outside a live battle; the setup /
//! teardown lifecycle systems run unconditionally `around` the band (setup creates the
//! witness, teardown removes it) — see [`BattleSimPlugin`]. [`BattleInProgress`] is an
//! explicit, intentional "a battle is active" tag — it replaced the incidental
//! [`OccupancyGrid`](crate::occupancy::OccupancyGrid)-as-gate proxy (GTW-212), but spans
//! the exact same battle-active window
//! ([`OccupancyGrid`](crate::occupancy::OccupancyGrid) remains a
//! [`setup_battle`](crate::situation::setup_battle) resource, just no longer the gate
//! witness), so the gated span — and behavior — is unchanged.
//!
//! The setup system drains [`SetupBattleRequested`] and per message inserts a
//! [`SimRng`](crate::rng::SimRng) seeded from the message's
//! [`BattleSeed`](crate::rng::BattleSeed) and runs
//! [`setup_battle`](crate::situation::setup_battle), emitting [`BattleReady`] (and
//! inserting the [`BattleInProgress`] gate witness) only on success (`Err` is
//! `error!`-logged with NO [`BattleReady`] / NO witness — fail-closed, no
//! `unwrap`/`expect`/`panic`). The teardown system drains [`TeardownBattleRequested`]
//! and removes the battle-lifetime resources ([`SimRng`](crate::rng::SimRng) + the four
//! [`setup_battle`](crate::situation::setup_battle)-inserted grids + the
//! [`BattleInProgress`] witness); it NEVER touches
//! [`CombatTuning`](crate::tuning::CombatTuning) — that is E10.4's persistent `Load`
//! resource. Render-free: no renderer, window, presenter, or pixel.

mod messages;
mod outcome;
mod plugin;
mod resources;
mod setup;

#[cfg(test)]
mod test;

pub use messages::{
    BattleLost, BattleReady, BattleWon, SetupBattleRequested, TeardownBattleRequested,
};
pub use outcome::check_outcome;
pub use plugin::BattleSimPlugin;
pub use resources::{BattleInProgress, BattleRoster, PlayerFaction};
pub use setup::{setup_battle_on_request, teardown_battle_on_request};
