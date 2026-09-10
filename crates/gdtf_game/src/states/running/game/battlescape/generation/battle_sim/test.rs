use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    level::{PrefabRegistry, ThemeName, ThemeUuid, UuidThemeRegistry},
    procgen::StagedProcgen,
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
    terrain::def::TerrainDefRegistry,
    test_support::fixtures,
};
use gdtf_content_families::situation::LoadedSituation;

use super::{
    context::BattleGenerationContext,
    preplaced::PreplacedGangers,
    resolved::ResolvedBattleSeed,
    systems::{advance_battle_generation, begin_battle_generation, finish_battle_generation},
};
use crate::states::load::hot_reload_test_support::capture_logs;

// The seed both fixtures pin, so the resolved seed never comes off the wall clock.
const FIXTURE_SEED: u64 = 0x0582_0005;

fn situation_themed(theme: ThemeUuid) -> Situation {
    let mut situation = Situation::new();
    situation.map.theme = theme;
    situation
}

fn procgen_app_with_absent_theme(theme: ThemeUuid) -> App {
    let mut app = App::new();
    app.add_message::<SetupBattleRequested>();
    app.insert_resource(LoadedSituation::new(situation_themed(theme)));
    app.insert_resource(PrefabRegistry::default());
    app.insert_resource(UuidThemeRegistry::default());
    app.insert_resource(TerrainDefRegistry::default());
    app.insert_resource(ContentIntegrityReport::default());
    app.insert_resource(BattleSeed::new(FIXTURE_SEED));
    app
}

// The same fixture with no catalogs at all, so `ProcgenContent::staged` resolves nothing.
fn procgen_app_without_catalogs(theme: ThemeUuid) -> App {
    let mut app = App::new();
    app.add_message::<SetupBattleRequested>();
    app.insert_resource(LoadedSituation::new(situation_themed(theme)));
    app.insert_resource(ContentIntegrityReport::default());
    app.insert_resource(BattleSeed::new(FIXTURE_SEED));
    app
}

fn theme_named(name: &str) -> ThemeUuid {
    ThemeUuid::from_legacy_theme(&ThemeName::new(name.to_owned()))
}

// Every request written so far, taken off the queue.
fn drain_requests(app: &mut App) -> Vec<SetupBattleRequested> {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .drain()
        .collect()
}

// The placements every request written so far carried, taken off the queue.
fn drain_placements(app: &mut App) -> Vec<Vec<PlacedGanger>> {
    drain_requests(app)
        .into_iter()
        .map(|request| request.placements)
        .collect()
}

// Run the one generation route end to end: begin, advance, finish.
fn run_generation(app: &mut App) {
    let began = app.world_mut().run_system_once(begin_battle_generation);
    assert!(began.is_ok(), "begin_battle_generation must run cleanly");
    let advanced = app.world_mut().run_system_once(advance_battle_generation);
    assert!(
        advanced.is_ok(),
        "advance_battle_generation must run cleanly"
    );
    let finished = app.world_mut().run_system_once(finish_battle_generation);
    assert!(
        finished.is_ok(),
        "finish_battle_generation must run cleanly"
    );
}

#[test]
fn begin_records_the_seed_the_battle_was_generated_from() {
    let mut app = procgen_app_with_absent_theme(theme_named("begin-seed-fixture"));

    let began = app.world_mut().run_system_once(begin_battle_generation);
    assert!(began.is_ok(), "begin_battle_generation must run cleanly");

    assert_eq!(
        app.world().get_resource::<ResolvedBattleSeed>().copied(),
        Some(ResolvedBattleSeed::new(BattleSeed::new(FIXTURE_SEED))),
        "begin_battle_generation must record the BattleSeed override as the ResolvedBattleSeed, \
         which is the replay handle every later reader takes",
    );
    assert!(
        app.world()
            .get_resource::<BattleGenerationContext>()
            .is_some(),
        "and it must leave the context the finish reads back",
    );
}

#[test]
fn unresolved_catalogs_set_the_battle_up_from_the_authored_situation() {
    let theme = theme_named("no-catalog-fixture");
    let mut app = procgen_app_without_catalogs(theme);

    run_generation(&mut app);

    assert!(
        app.world().get_resource::<StagedProcgen>().is_none(),
        "with no catalogs resolved there is nothing to generate from, so no driver is inserted",
    );
    let requests = drain_requests(&mut app);
    let carried: Vec<(ThemeUuid, usize)> = requests
        .iter()
        .map(|request| (request.situation.map.theme, request.placements.len()))
        .collect();
    assert_eq!(
        carried,
        vec![(theme, 0)],
        "the no-driver case must request the battle from the AUTHORED situation with no \
         placements",
    );
    assert!(
        app.world()
            .resource::<ContentIntegrityReport>()
            .findings()
            .is_empty(),
        "and it must record no finding: unresolved catalogs are not a degraded generation",
    );
}

#[test]
fn preplaced_gangers_are_the_placements_the_setup_request_carries() {
    let (_situation, settled) = fixtures::two_ganger();
    let mut app = procgen_app_with_absent_theme(theme_named("preplaced-fixture"));

    run_generation(&mut app);
    assert_eq!(
        drain_placements(&mut app),
        vec![Vec::new()],
        "with no PreplacedGangers the request carries the procgen outcome's own placements, and \
         this situation authors no roster to deploy",
    );

    app.insert_resource(PreplacedGangers::new(settled.clone()));
    run_generation(&mut app);
    assert_eq!(
        drain_placements(&mut app),
        vec![settled],
        "PreplacedGangers must replace the outcome's placements, so a caller that settled the \
         cells itself gets those cells spawned",
    );
}

#[test]
fn absent_theme_procgen_fallback_warns_and_lands_on_the_report() {
    let mut app = procgen_app_with_absent_theme(theme_named("absent-theme-fixture"));

    let began = app.world_mut().run_system_once(begin_battle_generation);
    assert!(began.is_ok(), "begin_battle_generation must run cleanly");

    let captured = capture_logs(|| {
        let advanced = app.world_mut().run_system_once(advance_battle_generation);
        assert!(
            advanced.is_ok(),
            "advance_battle_generation must run cleanly"
        );
        let finished = app.world_mut().run_system_once(finish_battle_generation);
        assert!(
            finished.is_ok(),
            "finish_battle_generation must run cleanly"
        );
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("procgen could not assemble a level")),
        "the empty-board fallback must warn! loudly; captured: {captured:?}",
    );

    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        report.findings().iter().any(|finding| matches!(
            finding,
            ContentFinding::DegradedFallback { context, .. }
                if context.contains("empty-board fallback")
        )),
        "the empty-board fallback must land on the ContentIntegrityReport; findings: {:?}",
        report.findings(),
    );

    let requests = app
        .world()
        .resource::<Messages<SetupBattleRequested>>()
        .len();
    assert_eq!(
        requests, 1,
        "the fallback must still request the battle setup (authored terrain as-is)",
    );
}
