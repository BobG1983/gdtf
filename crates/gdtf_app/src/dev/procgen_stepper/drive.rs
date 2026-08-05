use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    procgen::{ProcgenTuning, StagedProcgen},
    rng::BattleSeed,
    situation::Situation,
};

use super::{
    clock::AutoStepClock,
    commands::{AutoRunning, AutoStepTimer, PendingStepCommand, StepCommand},
};
use crate::states::{
    LoadedSituation,
    running::game::battlescape::generation::battle_sim::{
        ProcgenContent, ResolvedBattleSeed, deploy_over_generated, outcome_from_packing_error,
        resolve_root_seed,
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
    commands.insert_resource(ResolvedBattleSeed::new(seed));
    commands.insert_resource(StagedProcgen::new(seed, authored.theme, authored.grid_size));
    commands.insert_resource(ProcgenStepperContext { authored, seed });
    commands.insert_resource(PendingStepCommand::default());
    commands.insert_resource(AutoRunning::default());
    commands.insert_resource(AutoStepTimer::default());
}

pub(super) fn advance_stepper_drive(
    driver: Option<ResMut<StagedProcgen>>,
    pending: Option<ResMut<PendingStepCommand>>,
    content: ProcgenContent,
    mut clock: AutoStepClock,
) {
    let (Some(mut driver), Some(mut pending)) = (driver, pending) else {
        return;
    };
    if driver.is_done() {
        return;
    }
    let default_tuning = ProcgenTuning::default();
    let Some(registries) = content.staged(&default_tuning) else {
        return;
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

    if *clock.stage_due() {
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
