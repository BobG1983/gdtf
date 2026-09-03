//! the PREFAB placement edge joins the editor's authoring-time validation,
//! and a prefab-registry change re-arms it.
use bevy::prelude::DetectChangesMut as _;
use gdtf_assets::{ContentIntegrityReport, ContentValidationDone, ReferenceKeyScheme};
use gdtf_battle_sim::level::PrefabRegistry;

use crate::harness::{advance_to_published, editor_app_on_fixture_root, has_dangling_ref};

const DANGLING_PLACEMENT: &str = "00000000-0000-0000-0000-063000000f01";

// Few enough that the re-arm fails rather than hangs: `advance_until` has no cap.
const REARM_UPDATES: usize = 8;

#[test]
fn a_prefab_placing_an_unknown_terrain_def_is_reported_in_the_editor() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };

    assert!(
        has_dangling_ref(
            report,
            "TerrainDefRegistry",
            DANGLING_PLACEMENT,
            ReferenceKeyScheme::Uuid,
        ),
        "the fixture prefab's dangling placed terrain UUID must be reported at authoring \
         time; report: {:?}",
        report.findings(),
    );
}

#[test]
fn a_prefab_registry_change_rearms_editor_validation() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    app.world_mut()
        .resource_mut::<PrefabRegistry>()
        .set_changed();

    let mut cleared = false;
    for _ in 0..REARM_UPDATES {
        app.update();
        if app
            .world()
            .get_resource::<ContentValidationDone>()
            .is_none()
        {
            cleared = true;
            break;
        }
    }

    assert!(
        cleared,
        "a changed PrefabRegistry must re-arm validation — ContentValidationDone was present \
         after every one of {REARM_UPDATES} updates",
    );
}
