use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
    procgen::{ProcgenAdvance, ProcgenTuning, StagedProcgen},
    rng::BattleSeed,
    situation::Situation,
};
use gdtf_content_families::situation::LoadedSituation;

use super::{
    content::ProcgenContent,
    context::BattleGenerationContext,
    deploy::deploy_over_generated,
    preplaced::PreplacedGangers,
    procgen::{ProcgenOutcome, outcome_from_packing_error},
    resolved::ResolvedBattleSeed,
    seed::resolve_root_seed,
};
use crate::states::running::game::battlescape::generation::resources::GenerationComplete;

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn begin_battle_generation(
    loaded: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
    content: ProcgenContent,
    mut commands: Commands,
) {
    let authored: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    let seed = seed_override.map_or_else(resolve_root_seed, |r| *r);
    commands.insert_resource(ResolvedBattleSeed::new(seed));
    info!(
        seed = *seed,
        "battle setup: resolved BattleSeed (RNG replay handle)"
    );
    let fallback_tuning = ProcgenTuning::default();
    if content.staged(&fallback_tuning).is_some() {
        commands.insert_resource(StagedProcgen::new(
            seed,
            authored.map.theme,
            authored.map.grid_size,
        ));
    }
    commands.insert_resource(BattleGenerationContext::new(authored, seed));
}

pub(crate) fn advance_battle_generation(
    driver: Option<ResMut<StagedProcgen>>,
    advance: Option<ResMut<ProcgenAdvance>>,
    content: ProcgenContent,
) {
    let Some(mut driver) = driver else {
        return;
    };
    if driver.is_done() {
        return;
    }
    let fallback_tuning = ProcgenTuning::default();
    let Some(registries) = content.staged(&fallback_tuning) else {
        return;
    };
    let Some(mut advance) = advance else {
        let _ran = driver.run_to_completion(registries);
        return;
    };
    match *advance {
        ProcgenAdvance::Hold => {}
        ProcgenAdvance::OneStage => {
            let _advanced = driver.advance(registries);
            *advance = ProcgenAdvance::Hold;
        }
        ProcgenAdvance::AllStages => {
            let _ran = driver.run_to_completion(registries);
        }
    }
}

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn finish_battle_generation(
    driver: Option<Res<StagedProcgen>>,
    context: Option<Res<BattleGenerationContext>>,
    preplaced: Option<Res<PreplacedGangers>>,
    report: Option<ResMut<ContentIntegrityReport>>,
    mut setup: MessageWriter<SetupBattleRequested>,
    mut commands: Commands,
) {
    let Some(context) = context else {
        return;
    };
    let Some(outcome) = settled_outcome(driver.as_deref(), &context) else {
        return;
    };
    let ProcgenOutcome {
        situation,
        placements,
        findings,
        deployment_error,
    } = outcome;

    if let Some(mut report) = report {
        for finding in findings {
            report.record(finding);
        }
    }

    if let Some(err) = &deployment_error {
        error!(
            "procgen could not deploy the roster into its deployment zone ({err}); the battle \
             will not set up (staying in Generation)"
        );
        clear_generation(&mut commands);
        return;
    }

    let placements = preplaced.map_or(placements, |settled| (**settled).clone());
    setup.write(SetupBattleRequested::new(
        situation,
        placements,
        context.seed(),
    ));
    clear_generation(&mut commands);
}

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn clear_battle_generation(
    mut commands: Commands,
) {
    clear_generation(&mut commands);
}

// The outcome the finish writes, or None while the driver still has stages left.
fn settled_outcome(
    driver: Option<&StagedProcgen>,
    context: &BattleGenerationContext,
) -> Option<ProcgenOutcome> {
    let Some(driver) = driver else {
        return Some(ProcgenOutcome {
            situation:        context.authored().clone(),
            placements:       Vec::new(),
            findings:         Vec::new(),
            deployment_error: None,
        });
    };
    if !driver.is_done() {
        return None;
    }
    if let Some(emitted) = driver.emitted() {
        return Some(deploy_over_generated(
            context.authored().clone(),
            emitted.clone(),
            context.seed(),
        ));
    }
    driver
        .failure()
        .map(|err| outcome_from_packing_error(context.authored().clone(), err))
}

fn clear_generation(commands: &mut Commands) {
    commands.remove_resource::<StagedProcgen>();
    commands.remove_resource::<BattleGenerationContext>();
}

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn gate_generation_complete(
    mut ready: MessageReader<BattleReady>,
    mut commands: Commands,
) {
    if ready.read().next().is_some() {
        commands.insert_resource(GenerationComplete);
    }
}

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn request_battle_teardown(
    mut teardown: MessageWriter<TeardownBattleRequested>,
) {
    teardown.write(TeardownBattleRequested);
}
