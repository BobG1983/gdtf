use bevy::{asset::AssetEvent, ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryName, InjuryRegistry, InjuryTables},
    severity::Severity,
};

use super::{
    super::redrive_injuries_on_asset_event,
    support::{add_def, add_folder, add_weighting, app, injury_def, weighting},
};
use crate::states::load::{
    resources::ActiveInjuriesFolderHandle, systems::resolve::hot_reload_test_support::capture_logs,
};

#[test]
fn modified_member_rebuilds_both_resources() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", InjuryCategory::Head, Severity::Critical) else {
        return;
    };
    let Some(w) = weighting(InjuryCategory::Head, &[], &[], &[("lost_eye", 4)]) else {
        return;
    };
    let def_h = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let w_h = add_weighting(&mut app, "content/injuries/weighting/head.weighting.ron", w);
    let w_id = w_h.id();
    let folder = add_folder(&mut app, &[def_h.untyped(), w_h.untyped()]);
    app.world_mut()
        .insert_resource(ActiveInjuriesFolderHandle::new(folder));
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut().insert_resource(InjuryTables::default());

    app.world_mut()
        .write_message(AssetEvent::Modified { id: w_id });
    app.update();

    let key = InjuryName::new("lost_eye".to_owned());
    let registry_has = app
        .world()
        .get_resource::<InjuryRegistry>()
        .is_some_and(|r| r.contains(&key));
    let tables_len = app
        .world()
        .get_resource::<InjuryTables>()
        .map(InjuryTables::len);
    assert!(
        registry_has,
        "the rebuild must repopulate the InjuryRegistry"
    );
    assert_eq!(
        tables_len,
        Some(1),
        "the rebuild must repopulate the InjuryTables (the Critical/Head bucket)",
    );
}

#[test]
fn injury_hot_reload_logs_an_info_line() {
    let mut app = app();
    let Some(eye) = injury_def("Lost Eye", InjuryCategory::Head, Severity::Critical) else {
        return;
    };
    let def_h = add_def(&mut app, "content/injuries/head/lost_eye.injury.ron", eye);
    let def_id = def_h.id();
    let folder = add_folder(&mut app, &[def_h.untyped()]);
    app.world_mut()
        .insert_resource(ActiveInjuriesFolderHandle::new(folder));
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut().insert_resource(InjuryTables::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: def_id });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_injuries_on_asset_event);
        assert!(result.is_ok(), "the redrive system must run cleanly");
    });

    assert!(
        captured.iter().any(|line| {
            line.contains("injury hot-reload")
                && line.contains("InjuryRegistry")
                && line.contains("InjuryTables")
        }),
        "the injury hot-reload must emit an info! line naming both resources; captured: {captured:?}",
    );
}
