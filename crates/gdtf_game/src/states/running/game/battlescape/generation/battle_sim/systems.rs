use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
    rng::BattleSeed,
    situation::Situation,
};
use gdtf_content_families::situation::LoadedSituation;

use super::{
    content::ProcgenContent, procgen::procgen_battle_situation, resolved::ResolvedBattleSeed,
    seed::resolve_root_seed,
};
use crate::states::running::game::battlescape::generation::resources::GenerationComplete;

pub(in crate::states::running::game::battlescape::generation::battle_sim) fn request_battle_setup(
    loaded: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
    content: ProcgenContent,
    report: Option<ResMut<ContentIntegrityReport>>,
    mut setup: MessageWriter<SetupBattleRequested>,
    mut commands: Commands,
) {
    let authored: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    let seed = seed_override.map_or_else(resolve_root_seed, |r| *r);
    commands.insert_resource(ResolvedBattleSeed::new(seed));
    info!(
        seed = *seed,
        "battle setup: resolved BattleSeed (RNG replay handle)"
    );
    let outcome = procgen_battle_situation(authored, content.registries(), seed);
    if let Some(mut report) = report {
        for finding in outcome.findings {
            report.record(finding);
        }
    }
    if let Some(err) = &outcome.deployment_error {
        error!(
            "procgen could not deploy the roster into its deployment zone ({err}); the battle \
             will not set up (staying in Generation)"
        );
        return;
    }
    setup.write(SetupBattleRequested::new(outcome.situation, seed));
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
