//! Test for the input-band ordering (relocated from `lib.rs`, GTW-201).

use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::{InputSystems, plugin::build::GdtfBattleInputPlugin};

/// A marker pushed into the shared order log by the band probes — distinguishes the
/// input band from the sim band so the test can read their relative run order.
///
/// Test-only; the framework plumbing carve-out — a tiny enum the probe systems push to
/// record which band ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeBand {
    /// Pushed by the probe in [`InputSystems::Gather`].
    Input,
    /// Pushed by the probe in [`SimSystems::Simulate`].
    Sim,
}

/// The shared order-recording resource the two probe systems append to.
///
/// A `Vec` log (test-only framework plumbing) recording the order the input-band and
/// sim-band probes ran within one `Update`, so the assert can read input-before-sim.
#[derive(bevy::prelude::Resource, Default)]
struct OrderLog(Vec<ProbeBand>);

/// Probe in the input band: records that [`InputSystems::Gather`] ran.
fn probe_input(mut log: bevy::prelude::ResMut<OrderLog>) {
    log.0.push(ProbeBand::Input);
}

/// Probe in the sim band: records that [`SimSystems::Simulate`] ran.
fn probe_sim(mut log: bevy::prelude::ResMut<OrderLog>) {
    log.0.push(ProbeBand::Sim);
}

/// AC2 — the input band runs BEFORE the sim band within one `Update` (the behavioral
/// order pin).
///
/// Builds a headless `MinimalPlugins` app, adds the real [`GdtfBattleInputPlugin`]
/// (which `configure_sets(Update, InputSystems::Gather.before(SimSystems::Simulate))`),
/// and registers two test-owned probes against a shared [`OrderLog`]: one
/// `.in_set(InputSystems::Gather)` and one `.in_set(SimSystems::Simulate)`. The test owns
/// the sim set's existence via its own `configure_sets(Update, SimSystems::Simulate)` so no
/// resource-requiring sim system is dragged in. After ONE `app.update()`, the log shows the
/// input marker BEFORE the sim marker.
///
/// Pin-discriminating: remove the plugin's
/// `InputSystems::Gather.before(SimSystems::Simulate)` configure and the two bands become
/// unordered (`bevy-traps.md` #3). Driven from the test body (`bevy-traps.md` #7 carve-out).
#[test]
fn input_band_runs_before_sim_band() {
    use bevy::prelude::{App, IntoScheduleConfigs, MinimalPlugins, Update};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .init_resource::<OrderLog>()
        // The test owns the sim set's existence (the sanctioned alternative to adding
        // `OccupancyMaintenancePlugin`, whose member systems would need battle-scoped
        // resources). `configure_sets` accumulates (`bevy-traps.md` #5).
        .configure_sets(Update, SimSystems::Simulate)
        .add_systems(Update, probe_input.in_set(InputSystems::Gather))
        .add_systems(Update, probe_sim.in_set(SimSystems::Simulate));

    app.update();

    let log = &app.world().resource::<OrderLog>().0;
    let input_at = log.iter().position(|b| *b == ProbeBand::Input);
    let sim_at = log.iter().position(|b| *b == ProbeBand::Sim);
    let (Some(input_at), Some(sim_at)) = (input_at, sim_at) else {
        assert_eq!(
            (input_at.is_some(), sim_at.is_some()),
            (true, true),
            "both band probes must have run once in the single update",
        );
        return;
    };
    assert!(
        input_at < sim_at,
        "the input band must run BEFORE the sim band within one Update \
         (log: {log:?})",
    );
}
