//! GTW-582 C5 pin: the procgen **empty-board fallback is a LAST RESORT and
//! never silent** — a battle request whose theme UUID is absent from the
//! (present) registries cannot assemble a level, so the REAL
//! [`request_battle_setup`](super::systems::request_battle_setup) system must
//! (a) `warn!` (asserted through the shared, poison-proof `CaptureLayer`
//! scaffold — the GTW-494 recipe) and (b) record the degradation on the
//! [`ContentIntegrityReport`].

use bevy::{app::App, ecs::system::RunSystemOnce, prelude::*};
use gdtf_assets::{ContentFinding, ContentIntegrityReport};
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    level::{PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    rng::BattleSeed,
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};

use super::systems::request_battle_setup;
use crate::states::load::{LoadedSituation, hot_reload_test_support::capture_logs};

/// Build the minimal world the REAL `request_battle_setup` runs in: the three
/// procgen registries PRESENT (but not containing the requested theme), the
/// authored situation naming that absent theme, a deterministic seed override,
/// the integrity report, and the `SetupBattleRequested` message buffer.
fn procgen_app_with_absent_theme(theme: ThemeUuid) -> App {
    let mut app = App::new();
    app.add_message::<SetupBattleRequested>();
    let mut situation = Situation::new();
    situation.theme = theme;
    app.insert_resource(LoadedSituation::new(situation));
    // The registries are PRESENT but hold nothing for this theme — the genuine
    // absent-theme case (NOT the missing-registry harness early-out).
    app.insert_resource(PrefabRegistry::default());
    app.insert_resource(UuidThemeRegistry::default());
    app.insert_resource(TerrainDefRegistry::default());
    app.insert_resource(ContentIntegrityReport::default());
    // Deterministic seed override, so the run never touches env / wall clock.
    app.insert_resource(BattleSeed::new(0x0582_0005));
    app
}

/// C5: an absent theme UUID drives the empty-board fallback — the warn! fires
/// (captured through the real tracing path) AND the report carries the
/// `DegradedFallback` finding. The fallback still writes the setup request
/// (playable-but-degraded, never a strand).
#[test]
fn absent_theme_procgen_fallback_warns_and_lands_on_the_report() {
    // A deterministic theme UUID deliberately ABSENT from the (present, empty)
    // registries — the "procgen request with an absent theme UUID" shape.
    let theme = ThemeUuid::from_legacy_theme("gtw-582-absent-theme");
    let mut app = procgen_app_with_absent_theme(theme);

    let captured = capture_logs(|| {
        let result = app.world_mut().run_system_once(request_battle_setup);
        assert!(result.is_ok(), "request_battle_setup must run cleanly");
    });

    // (a) the warn! is LOUD on the real code path (no log assertion without the
    // capture scaffold — the GTW-494 poison-proof recipe).
    assert!(
        captured
            .iter()
            .any(|line| line.contains("procgen could not assemble a level")),
        "the empty-board fallback must warn! loudly; captured: {captured:?}",
    );

    // (b) the degradation is ON THE RECORD: a DegradedFallback finding naming
    // the empty-board fallback.
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

    // The fallback is playable-but-degraded, never a strand: the setup request
    // was still written (with the authored — empty — terrain).
    let requests = app
        .world()
        .resource::<Messages<SetupBattleRequested>>()
        .len();
    assert_eq!(
        requests, 1,
        "the fallback must still request the battle setup (authored terrain as-is)",
    );
}
