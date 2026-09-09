use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    level::{PrefabRegistry, ThemeName, ThemeUuid, UuidThemeRegistry},
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
    terrain::def::TerrainDefRegistry,
    test_support::fixtures,
};
use gdtf_content_families::situation::LoadedSituation;

use super::{preplaced::PreplacedGangers, systems::request_battle_setup};
use crate::states::load::hot_reload_test_support::capture_logs;

fn procgen_app_with_absent_theme(theme: ThemeUuid) -> App {
    let mut app = App::new();
    app.add_message::<SetupBattleRequested>();
    let mut situation = Situation::new();
    situation.map.theme = theme;
    app.insert_resource(LoadedSituation::new(situation));
    app.insert_resource(PrefabRegistry::default());
    app.insert_resource(UuidThemeRegistry::default());
    app.insert_resource(TerrainDefRegistry::default());
    app.insert_resource(ContentIntegrityReport::default());
    app.insert_resource(BattleSeed::new(0x0582_0005));
    app
}

// The placements every request written so far carried, taken off the queue.
fn drain_placements(app: &mut App) -> Vec<Vec<PlacedGanger>> {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .drain()
        .map(|request| request.placements)
        .collect()
}

// Run the setup request system once.
fn request_once(app: &mut App) {
    let result = app.world_mut().run_system_once(request_battle_setup);
    assert!(result.is_ok(), "request_battle_setup must run cleanly");
}

#[test]
fn preplaced_gangers_are_the_placements_the_setup_request_carries() {
    let theme = ThemeUuid::from_legacy_theme(&ThemeName::new("preplaced-fixture".to_owned()));
    let (_situation, settled) = fixtures::two_ganger();
    let mut app = procgen_app_with_absent_theme(theme);

    request_once(&mut app);
    assert_eq!(
        drain_placements(&mut app),
        vec![Vec::new()],
        "with no PreplacedGangers the request carries the procgen outcome's own placements, and \
         this situation authors no roster to deploy",
    );

    app.insert_resource(PreplacedGangers::new(settled.clone()));
    request_once(&mut app);
    assert_eq!(
        drain_placements(&mut app),
        vec![settled],
        "PreplacedGangers must replace the outcome's placements, so a caller that settled the \
         cells itself gets those cells spawned",
    );
}

#[test]
fn absent_theme_procgen_fallback_warns_and_lands_on_the_report() {
    let theme = ThemeUuid::from_legacy_theme(&ThemeName::new("absent-theme-fixture".to_owned()));
    let mut app = procgen_app_with_absent_theme(theme);

    let captured = capture_logs(|| {
        let result = app.world_mut().run_system_once(request_battle_setup);
        assert!(result.is_ok(), "request_battle_setup must run cleanly");
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
