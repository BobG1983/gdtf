#[path = "load_suite/gate.rs"]
mod gate;

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::{
    armor::BodyPart,
    injuries::{DamageContext, InjuryName, InjuryRegistry, InjuryTables},
    severity::Severity,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until, advance_until_resource_exists,
};

const TRANSITION_BUDGET: u32 = 32;

const LOAD_SAFETY_NET: u32 = 10_000;

#[test]
fn injuries_loader_no_ops_cleanly_without_asset_server() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.update();
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no AssetServer the kick-off must no-op and the machine rests in Load, not panic",
    );

    gate::seed_full_load_gate(&mut app);

    let released = advance_until(&mut app, load_released, TRANSITION_BUDGET);
    assert!(
        released,
        "with a GdtfTheme + CombatTuning + GangerStatTuning + WeaponRegistry + ArmorRegistry \
         + InjuryRegistry + LoadedSituation present, Load must release to \
         Intro (or beyond) within {TRANSITION_BUDGET} updates; last observed AppState was {:?}",
        app_state(&app),
    );
}

#[test]
fn load_does_not_leave_without_an_injury_registry() {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    gate::seed_gate_except::<InjuryRegistry>(&mut app);

    let left_load = advance_until(
        &mut app,
        |app| app_state(app) != AppState::Load,
        TRANSITION_BUDGET,
    );

    assert!(
        !left_load,
        "Load must NOT leave while the InjuryRegistry is absent; it left to {:?}",
        app_state(&app),
    );
    assert_eq!(
        app_state(&app),
        AppState::Load,
        "with no InjuryRegistry present, the machine stays in Load (the injuries folder must \
         be verified loaded before Load exits — a battle never starts injury-less)",
    );
}

#[test]
fn real_asset_resolves_injury_registry_and_tables() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<InjuryRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<InjuryRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved InjuryRegistry must carry the authored (non-empty) injuries",
        );
        assert!(
            registry
                .def(&InjuryName::new("scalp_graze".to_owned()))
                .is_some(),
            "the registry must hold the `scalp_graze` injury (keyed by \
             head/scalp_graze.injury.ron's stem)",
        );
        assert!(
            registry
                .def(&InjuryName::new("lost_eye".to_owned()))
                .is_some(),
            "the registry must hold the `lost_eye` injury (keyed by \
             head/lost_eye.injury.ron's stem)",
        );
    }

    if let Some(tables) = app.world().get_resource::<InjuryTables>() {
        assert!(
            !tables.is_empty(),
            "the resolved InjuryTables must carry the authored (non-empty) weighting buckets",
        );
        assert!(
            tables
                .table(BodyPart::Head, DamageContext::Ranged, Severity::Minor)
                .is_some(),
            "the (Head, Minor) bucket must be populated from head.weighting.ron's `minor` list \
             (the scalp_graze row)",
        );
        assert!(
            tables
                .table(BodyPart::Head, DamageContext::Ranged, Severity::Critical)
                .is_some(),
            "the (Head, Critical) bucket must be populated from head.weighting.ron's `critical` \
             list (the lost_eye row)",
        );

        for part in [
            BodyPart::Head,
            BodyPart::Torso,
            BodyPart::LeftArm,
            BodyPart::RightArm,
            BodyPart::LeftLeg,
            BodyPart::RightLeg,
        ] {
            let any_bucket = [Severity::Minor, Severity::Major, Severity::Critical]
                .into_iter()
                .any(|sev| tables.table(part, DamageContext::Ranged, sev).is_some());
            assert!(
                any_bucket,
                "the {part:?} category pool must have at least one rollable bucket (every category non-empty)",
            );
        }
        assert_eq!(
            tables.table(BodyPart::LeftArm, DamageContext::Ranged, Severity::Major),
            tables.table(BodyPart::RightArm, DamageContext::Ranged, Severity::Major),
            "both arms must resolve the IDENTICAL shared Arm (Major) bucket",
        );
        assert_eq!(
            tables.table(BodyPart::LeftLeg, DamageContext::Ranged, Severity::Major),
            tables.table(BodyPart::RightLeg, DamageContext::Ranged, Severity::Major),
            "both legs must resolve the IDENTICAL shared Leg (Major) bucket",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once every folder \
         (incl. injuries) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<InjuryRegistry>().is_some(),
        "an InjuryRegistry must be present when Load reaches Intro (the gate waited for it)",
    );
    assert!(
        app.world().get_resource::<InjuryTables>().is_some(),
        "the InjuryTables resolved alongside the registry (resolve_injuries built BOTH)",
    );
}
