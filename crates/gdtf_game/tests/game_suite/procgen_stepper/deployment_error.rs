use bevy::{app::App, state::state::NextState};
use cobalt_test_utils::advance_until;
use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_battle_sim::procgen::StagedProcgen;
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{BattleScapeState, RunningState};

use super::harness::{FIXED_SEED, app_ready_for_battle, battlescape_state};

/// Frames the failed generation is given to prove it records its finding once.
const HOLD_FRAMES: u32 = 8;

/// The context `deploy_over_generated` files its deployment failure under.
const DEPLOYMENT_CONTEXT: &str = "procgen roster deployment";

// Repeat the authored roster until it cannot fit the board, so every zone is too small.
fn overfill_the_roster(app: &mut App) {
    let loaded = app.world_mut().get_resource_mut::<LoadedSituation>();
    assert!(
        loaded.is_some(),
        "the real Load flow must resolve the authored situation",
    );
    let Some(mut loaded) = loaded else {
        return;
    };
    let situation = loaded.situation_mut();
    let cells = usize::from(*situation.map.grid_size.width())
        * usize::from(*situation.map.grid_size.height());
    let authored = situation.combatants.rosters.clone();
    assert!(
        !authored.is_empty(),
        "the authored situation must carry a roster for this case to overfill",
    );
    if authored.is_empty() {
        return;
    }
    while situation.combatants.rosters.len() <= cells {
        situation
            .combatants
            .rosters
            .extend(authored.iter().cloned());
    }
}

fn deployment_findings(app: &App) -> usize {
    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the asset families must have inserted a ContentIntegrityReport",
    );
    report.map_or(0, |report| {
        report
            .findings()
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    ContentFinding::DegradedFallback { context, .. }
                        if context.contains(DEPLOYMENT_CONTEXT)
                )
            })
            .count()
    })
}

#[test]
fn a_roster_that_cannot_stand_records_its_finding_once() {
    let mut app = app_ready_for_battle(FIXED_SEED);
    overfill_the_roster(&mut app);
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::Generation)
    });

    for _ in 0..HOLD_FRAMES {
        app.update();
    }

    assert_eq!(
        deployment_findings(&app),
        1,
        "a generation that cannot deploy its roster must record ONE finding and stop, not one \
         per frame for as long as the machine sits in Generation",
    );
    assert!(
        app.world().get_resource::<StagedProcgen>().is_none(),
        "and the failed finish must clear the driver away",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::Generation),
        "the battle does not set up, so the machine stays in Generation",
    );
}
