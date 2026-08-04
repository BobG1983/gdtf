use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{ProcgenTuning, StagedProcgen, StagedProcgenRegistries},
    rng::BattleSeed,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};

use super::commands::{AutoRunning, AutoStepTimer, PendingStepCommand, StepCommand};
use crate::states::{
    LoadedSituation,
    running::game::battlescape::generation::battle_sim::{
        deploy_over_generated, outcome_from_packing_error, resolve_root_seed,
    },
};

#[derive(Resource, Debug, Clone)]
pub(super) struct ProcgenStepperContext {
    authored: Situation,
    seed:     BattleSeed,
}

pub(super) fn engage_stepper(
    loaded: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
    mut commands: Commands,
) {
    let authored: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    let seed = seed_override.map_or_else(resolve_root_seed, |r| *r);
    commands.insert_resource(StagedProcgen::new(seed, authored.theme, authored.grid_size));
    commands.insert_resource(ProcgenStepperContext { authored, seed });
    commands.insert_resource(PendingStepCommand::default());
    commands.insert_resource(AutoRunning::default());
    commands.insert_resource(AutoStepTimer::default());
}

#[expect(
    clippy::too_many_arguments,
    reason = "one system drives the whole per-frame advance decision: the driver + the two \
              latch resources + the live registries it borrows for its ONE stage — each a \
              distinct Bevy SystemParam (Option<Res<_>> for every Load-state resource, \
              mirroring request_battle_setup's own justified exception)"
)]
pub(super) fn advance_stepper_drive(
    driver: Option<ResMut<StagedProcgen>>,
    pending: Option<ResMut<PendingStepCommand>>,
    auto_running: Option<Res<AutoRunning>>,
    timer: Option<ResMut<AutoStepTimer>>,
    time: Res<Time>,
    prefabs: Option<Res<PrefabRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    terrain_defs: Option<Res<TerrainDefRegistry>>,
    tuning: Option<Res<ProcgenTuning>>,
) {
    let (Some(mut driver), Some(mut pending)) = (driver, pending) else {
        return;
    };
    if driver.is_done() {
        return;
    }
    let (Some(prefabs), Some(themes), Some(terrain_defs)) = (
        prefabs.as_deref(),
        themes.as_deref(),
        terrain_defs.as_deref(),
    ) else {
        return;
    };
    let default_tuning = ProcgenTuning::default();
    let tuning = tuning.as_deref().unwrap_or(&default_tuning);
    let registries = StagedProcgenRegistries {
        prefabs,
        themes,
        terrain_defs,
        tuning,
    };

    if let Some(command) = pending.take() {
        match command {
            StepCommand::Next => {
                let _advanced = driver.advance(registries);
            }
            StepCommand::Skip => {
                let _ran = driver.run_to_completion(registries);
            }
        }
        return;
    }

    let (Some(auto_running), Some(mut timer)) = (auto_running, timer) else {
        return;
    };
    if !auto_running.is_running() {
        return;
    }
    timer.tick(time.delta());
    if timer.just_finished() {
        let _advanced = driver.advance(registries);
    }
}

pub(super) fn finish_stepper_drive(
    driver: Option<Res<StagedProcgen>>,
    context: Option<Res<ProcgenStepperContext>>,
    report: Option<ResMut<ContentIntegrityReport>>,
    mut setup: MessageWriter<SetupBattleRequested>,
    mut commands: Commands,
) {
    let (Some(driver), Some(context)) = (driver, context) else {
        return;
    };
    if !driver.is_done() {
        return;
    }

    let outcome = if let Some(emitted) = driver.emitted() {
        deploy_over_generated(context.authored.clone(), emitted.clone(), context.seed)
    } else if let Some(err) = driver.failure() {
        outcome_from_packing_error(context.authored.clone(), err)
    } else {
        return;
    };

    if let Some(mut report) = report {
        for finding in outcome.findings {
            report.record(finding);
        }
    }

    if let Some(err) = &outcome.deployment_error {
        error!(
            "procgen could not deploy the roster into its deployment zone ({err}); the \
             stepper-started battle will not set up (staying in Generation)"
        );
        remove_stepper_resources(&mut commands);
        return;
    }

    setup.write(SetupBattleRequested::new(outcome.situation, context.seed));

    remove_stepper_resources(&mut commands);
}

pub(super) fn cleanup_stepper_drive(mut commands: Commands) {
    remove_stepper_resources(&mut commands);
}

fn remove_stepper_resources(commands: &mut Commands) {
    commands.remove_resource::<StagedProcgen>();
    commands.remove_resource::<ProcgenStepperContext>();
    commands.remove_resource::<PendingStepCommand>();
    commands.remove_resource::<AutoRunning>();
    commands.remove_resource::<AutoStepTimer>();
}
