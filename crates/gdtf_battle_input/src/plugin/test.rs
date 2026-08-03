use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::{InputSystems, plugin::build::GdtfBattleInputPlugin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeBand {
        Input,
        Sim,
}

#[derive(bevy::prelude::Resource, Default)]
struct OrderLog(Vec<ProbeBand>);

fn probe_input(mut log: bevy::prelude::ResMut<OrderLog>) {
    log.0.push(ProbeBand::Input);
}

fn probe_sim(mut log: bevy::prelude::ResMut<OrderLog>) {
    log.0.push(ProbeBand::Sim);
}

#[test]
fn input_band_runs_before_sim_band() {
    use bevy::prelude::{App, IntoScheduleConfigs, MinimalPlugins, Update};

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .init_resource::<OrderLog>()
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
