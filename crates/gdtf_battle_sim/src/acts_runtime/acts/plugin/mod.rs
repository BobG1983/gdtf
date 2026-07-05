//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`](crate::acts::request::EndTurnRequested)
//! turn signal + the [`dispatch_end_turn`](crate::turn::dispatch_end_turn) turn-cycle engine — added in GTW-309).

use bevy::prelude::{App, Plugin};

mod acts;
mod messages;
mod reaction_suppression;
mod turn_clocks;

/// The **sim-acts registration unit** — registers the eight `*Requested` message buffers
/// and adds the eight per-act dispatch systems, every one in the `SimSystems::Simulate` set
/// in [`Update`](bevy::prelude::Update) (E10.2 / GTW-204; the seventh — [`MoveRequested`](crate::acts::request::MoveRequested) / [`dispatch_move`](crate::acts::dispatch_move) —
/// added in GTW-234; the eighth — [`ReloadRequested`](crate::acts::request::ReloadRequested) / [`dispatch_reload`](crate::acts::dispatch_reload) — in
/// GTW-275).
///
/// This slice CREATES this plugin — E10.0 lands only the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate)
/// [`SystemSet`](bevy::prelude::SystemSet) enum and its `configure_sets`; it builds no
/// plugin. In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s the eight `*Requested` input buffers
///   ([`FireRequested`](crate::acts::request::FireRequested), [`SetAimingRequested`](crate::acts::request::SetAimingRequested), [`SetStanceRequested`](crate::acts::request::SetStanceRequested),
///   [`SetFacingRequested`](crate::acts::request::SetFacingRequested), [`StabilizeDownedRequested`](crate::acts::request::StabilizeDownedRequested), [`ExecuteDownedRequested`](crate::acts::request::ExecuteDownedRequested),
///   [`MoveRequested`](crate::acts::request::MoveRequested), [`ReloadRequested`](crate::acts::request::ReloadRequested)) PLUS the GTW-290 output buffer
///   [`ShotFired`](crate::shot_fired::ShotFired) (emitted by the fire dispatch system) — exactly once each (`bevy-traps.md`
///   #5; an unregistered message buffer fails a
///   [`MessageReader`](bevy::prelude::MessageReader) /
///   [`MessageWriter`](bevy::prelude::MessageWriter)'s param validation, the
///   [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
///   precedent); and
/// - adds the eight dispatch systems to [`Update`](bevy::prelude::Update) in the `SimSystems::Simulate` set,
///   composing deterministically with the occupancy-maintenance systems already in that
///   set (`bevy-traps.md` #3).
///
/// It consumes E10.0's [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) set (it imports and uses it, never
/// redefines it) and does NOT call `configure_sets` — that is E10.0's job, run by
/// whichever plugin owns the set's configuration (the
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)).
/// Wiring this plugin into the `BattleRunning` lifecycle is E10.6, out of scope here.
///
/// **GTW-336 — the §9 bleed-out clock.** The plugin ALSO registers the
/// [`Bleeding`](crate::bleed::Bleeding) signal buffer and adds
/// [`tick_bleed`](crate::bleed::tick_bleed) in the `SimSystems::Simulate` set,
/// `.after(`[`dispatch_end_turn`](crate::turn::dispatch_end_turn)`)` and gated
/// `.run_if(`[`enemy_phase_started`](crate::bleed::enemy_phase_started)`)` — so the
/// bleed-out drain fires once per FULL ROUND, at the enemy-phase start
/// (`docs/combat/resolution.md` §9). Until this slice the drain was unit-test-only; this
/// is what makes Downed gangers actually bleed (and die to the clock) in a live battle and
/// what creates the `Messages<Bleeding>` buffer the presenter's `"Bleeding"` consequence
/// pop is gated on.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimActsPlugin;
impl Plugin for SimActsPlugin {
    fn build(&self, app: &mut App) {
        messages::register_messages(app);
        acts::wire_acts(app);
        turn_clocks::wire_turn_clocks(app);
        reaction_suppression::wire_reaction_suppression(app);
    }
}
