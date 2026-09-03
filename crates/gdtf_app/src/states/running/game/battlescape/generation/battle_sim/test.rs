use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    level::{PrefabRegistry, ThemeName, ThemeUuid, UuidThemeRegistry},
    rng::BattleSeed,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_families::situation::LoadedSituation;

use super::systems::request_battle_setup;
use crate::states::load::hot_reload_test_support::capture_logs;

fn procgen_app_with_absent_theme(theme: ThemeUuid) -> App {
    let mut app = App::new();
    app.add_message::<SetupBattleRequested>();
    let mut situation = Situation::new();
    situation.theme = theme;
    app.insert_resource(LoadedSituation::new(situation));
    app.insert_resource(PrefabRegistry::default());
    app.insert_resource(UuidThemeRegistry::default());
    app.insert_resource(TerrainDefRegistry::default());
    app.insert_resource(ContentIntegrityReport::default());
    app.insert_resource(BattleSeed::new(0x0582_0005));
    app
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
