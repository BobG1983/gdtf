//! Drives the staged procgen pipeline across frames while the stepper is engaged: engages
//! it `OnEnter(BattleScapeState::Generation)`, advances it from the latched command / the
//! Auto timer each `Update`, and finishes it — writing the SAME `SetupBattleRequested` the
//! normal path writes — once the drive completes.

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
        outcome_from_emitted, outcome_from_packing_error, resolve_root_seed,
    },
};

/// The authored inputs the staged drive needs to FINISH exactly like `request_battle_setup`
/// would: the authored [`Situation`] (merged over the generated terrain once the drive
/// completes) and the [`BattleSeed`] the battle's RNG is seeded from — the SAME one
/// [`StagedProcgen`]'s RNG was derived from.
///
/// `pub(super)`, not private: [`finish_stepper_drive`] is `pub(super)` (called from
/// `super::plugin`), so its `Option<Res<ProcgenStepperContext>>` parameter must be at least
/// as visible as the function itself (`private_interfaces`) — even though no OTHER module
/// ever spells this type's name.
#[derive(Resource, Debug, Clone)]
pub(super) struct ProcgenStepperContext {
    /// The authored battlefield (gangers / spawn / `player_faction` / fields) the generated
    /// terrain is merged over once the drive completes.
    authored: Situation,
    /// The per-battle seed — the SAME one the driver's RNG was derived from.
    seed:     BattleSeed,
}

/// `OnEnter(BattleScapeState::Generation)`, registered only while the stepper is enabled:
/// resolve the SAME authored situation + seed `request_battle_setup` would, start a fresh
/// [`StagedProcgen`] drive, and reset the command/Auto-run/timer state for this battle.
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

/// `Update`, registered only while the stepper is enabled: apply the latched command (Next /
/// Skip) if one is pending, else tick the Auto-run timer and advance one stage when it fires.
///
/// A pending command WINS over Auto this frame (checked first, and returns before ticking
/// the timer) so a Next/Skip press never double-advances alongside a same-frame Auto tick.
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

/// `Update`, registered only while the stepper is enabled, ordered `.after(advance_stepper_drive)`:
/// once the drive is done, finish it through the SAME merge / finding-conversion helpers
/// `request_battle_setup` uses, write the SAME `SetupBattleRequested`, and clear every
/// stepper resource this Generation span inserted.
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
        outcome_from_emitted(context.authored.clone(), emitted.clone())
    } else if let Some(err) = driver.failure() {
        outcome_from_packing_error(context.authored.clone(), err)
    } else {
        // `is_done()` guarantees `emitted()` or `failure()` is `Some` (the StagedProcgen
        // invariant) — this arm is unreachable in practice; return rather than panic/
        // unreachable! so a future driver change fails closed instead of aborting.
        return;
    };

    if let Some(mut report) = report {
        for finding in outcome.findings {
            report.record(finding);
        }
    }
    setup.write(SetupBattleRequested::new(outcome.situation, context.seed));

    remove_stepper_resources(&mut commands);
}

/// `OnExit(BattleScapeState::Generation)` safety net: clear every stepper resource this
/// Generation span may still hold, so nothing leaks into a re-entry (bevy-traps #1). A no-op
/// on the normal completion path (`finish_stepper_drive` already removed them).
pub(super) fn cleanup_stepper_drive(mut commands: Commands) {
    remove_stepper_resources(&mut commands);
}

/// The shared remove list [`finish_stepper_drive`] and [`cleanup_stepper_drive`] both apply.
fn remove_stepper_resources(commands: &mut Commands) {
    commands.remove_resource::<StagedProcgen>();
    commands.remove_resource::<ProcgenStepperContext>();
    commands.remove_resource::<PendingStepCommand>();
    commands.remove_resource::<AutoRunning>();
    commands.remove_resource::<AutoStepTimer>();
}
