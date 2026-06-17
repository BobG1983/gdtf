//! The sim's battle-lifecycle registration unit [`BattleSimPlugin`] — adding it wires
//! the WHOLE render-free combat runtime into a Bevy [`App`] (E10.5 / GTW-207 / GTW-237).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::SimActsPlugin,
    battle::{
        messages::{
            BattleLost, BattleReady, BattleWon, SetupBattleRequested, TeardownBattleRequested,
        },
        outcome::check_outcome,
        resources::BattleInProgress,
        setup::{setup_battle_on_request, teardown_battle_on_request},
    },
    occupancy_sync::{OccupancyMaintenancePlugin, SimSystems},
};

/// The sim's **battle-lifecycle registration unit** — adding this ONE plugin wires
/// the whole render-free combat runtime into a Bevy [`App`] (E10.5 / GTW-207).
///
/// In `build()` the plugin:
///
/// - adds [`OccupancyMaintenancePlugin`] (the E1.7 maintenance layer + the
///   [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) buffer; it OWNS the
///   [`SimSystems::Simulate`] `configure_sets`) and the E10.2 [`SimActsPlugin`] (the
///   six `*Requested` buffers + their per-act dispatch systems) — so the app adds
///   only THIS plugin to get the full sim runtime;
/// - [`add_message`](App::add_message)s the three lifecycle types
///   ([`SetupBattleRequested`] / [`TeardownBattleRequested`] / [`BattleReady`]) plus the
///   two GTW-237 outcome witnesses ([`BattleWon`] / [`BattleLost`]) exactly once each
///   (`bevy-traps.md` #5 — an unregistered buffer fails a
///   [`MessageReader`](bevy::prelude::MessageReader)'s param validation);
/// - adds the roster-grounded [`check_outcome`] census `.in_set(`[`SimSystems::Simulate`]`)`
///   — so it rides the same [`BattleInProgress`] gate as the bundled runtime (inert and
///   panic-free outside a live battle); and
/// - adds [`setup_battle_on_request`] + [`teardown_battle_on_request`] to [`Update`],
///   ordered around the [`SimSystems::Simulate`] band (setup `.before`, teardown
///   `.after`), so the battle-lifetime resources are created before the bundled
///   in-set systems read them and removed only after they have run.
///
/// **Battle-scoped gating of the bundled runtime.** The bundled [`SimActsPlugin`] +
/// [`OccupancyMaintenancePlugin`] systems (all `.in_set(SimSystems::Simulate)`) read
/// the battle-lifetime resources ([`OccupancyGrid`](crate::occupancy::OccupancyGrid) /
/// [`SurfaceGrid`](crate::surface::SurfaceGrid) /
/// [`CoverLedger`](crate::cover::CoverLedger) / [`SimRng`](crate::rng::SimRng) /
/// [`CombatTuning`](crate::tuning::CombatTuning))
/// unconditionally — a Bevy `Res<T>` whose resource is absent fails param validation
/// (`bevy-traps.md` #1). Those resources exist only between a setup and a teardown, so
/// `BattleSimPlugin` gates the whole `Simulate` band on
/// [`resource_exists`]`::<`[`BattleInProgress`]`>` (the setup-inserted "a battle is
/// active" witness — GTW-212's purpose-built tag, replacing the incidental
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid)-as-gate proxy) — a `run_if` skips a
/// set's member systems entirely (no param validation) when false, so the bundled runtime
/// stays inert (and panic-free) before the first battle and after teardown. The setup
/// inserts [`BattleInProgress`] on the same `Ok` path that inserts
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid) and the teardown removes it
/// alongside, so the gated span is identical — only the witness is explicit. The lifecycle
/// systems are deliberately NOT in that gated band: the setup system must run to CREATE the
/// witness (it would otherwise dead-gate itself), and the teardown system must run AFTER
/// the band to remove the witness — so both run unconditionally, ordered around the band.
///
/// The plugin names NO `gdtf_app` type: the battle lifecycle is driven entirely by
/// the three lifecycle messages, so the app sends triggers and reads the signal
/// without the sim ever depending back on the app.
#[derive(Debug, Default, Clone, Copy)]
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccupancyMaintenancePlugin)
            .add_plugins(SimActsPlugin)
            .add_message::<SetupBattleRequested>()
            .add_message::<TeardownBattleRequested>()
            .add_message::<BattleReady>()
            // The roster-grounded outcome witnesses the Simulate-band census emits (GTW-237;
            // bevy-traps.md #5 — an unregistered buffer fails a MessageReader's validation).
            .add_message::<BattleWon>()
            .add_message::<BattleLost>()
            // Gate the whole Simulate band (the bundled dispatch + occupancy systems)
            // on the setup-inserted BattleInProgress witness — the purpose-built
            // "a battle is active" tag (GTW-212), replacing the incidental OccupancyGrid
            // proxy — so the bundled runtime is inert + panic-free outside a live battle
            // (bevy-traps.md #1; conditions still accumulate across configure_sets #5).
            .configure_sets(
                Update,
                SimSystems::Simulate.run_if(resource_exists::<BattleInProgress>),
            )
            // The roster-grounded outcome census joins the gated Simulate band, so it is
            // INERT (and its Res<PlayerFaction>/Res<BattleRoster> reads panic-free) outside
            // a live battle — same window as BattleInProgress (GTW-237). No configure_sets
            // (the set is owned upstream by OccupancyMaintenancePlugin).
            .add_systems(Update, check_outcome.in_set(SimSystems::Simulate))
            // The lifecycle drivers run UNCONDITIONALLY (outside the gated band): setup
            // before the band (it creates the witness), teardown after (it removes it).
            .add_systems(
                Update,
                (
                    setup_battle_on_request.before(SimSystems::Simulate),
                    teardown_battle_on_request.after(SimSystems::Simulate),
                ),
            );
    }
}
