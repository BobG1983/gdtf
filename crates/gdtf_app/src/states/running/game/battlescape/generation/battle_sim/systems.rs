use bevy::prelude::*;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::ProcgenTuning,
    rng::BattleSeed,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};

use super::{
    procgen::{ProcgenRegistries, procgen_battle_situation},
    seed::resolve_root_seed,
};
use crate::states::{
    load::LoadedSituation, running::game::battlescape::generation::resources::GenerationComplete,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the params are the authored situation + seed override + the three UUID-keyed \
              procgen registries + the live tuning + the GTW-582 integrity report + the \
              setup writer — each a distinct Bevy SystemParam (Option<Res<_>> for the \
              Load-state resources); the sim's setup_battle_on_request precedent"
)]
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn request_battle_setup(
    loaded: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
    prefabs: Option<Res<PrefabRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    def_registry: Option<Res<TerrainDefRegistry>>,
    procgen_tuning: Option<Res<ProcgenTuning>>,
    report: Option<ResMut<ContentIntegrityReport>>,
    mut setup: MessageWriter<SetupBattleRequested>,
) {
    let authored: Situation = loaded.map_or_else(Situation::default, |loaded| (**loaded).clone());
    let seed = seed_override.map_or_else(resolve_root_seed, |r| *r);
    info!(
        seed = *seed,
        "battle setup: resolved BattleSeed (RNG replay handle)"
    );
    let registries = ProcgenRegistries {
        prefabs: prefabs.as_deref(),
        themes:  themes.as_deref(),
        terrain: def_registry.as_deref(),
        tuning:  procgen_tuning.as_deref(),
    };
    let outcome = procgen_battle_situation(authored, registries, seed);
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
