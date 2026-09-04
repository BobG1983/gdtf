//! the PREFAB placement edge joins the editor's authoring-time validation,
//! and a prefab-registry change re-arms it.
use bevy::prelude::DetectChangesMut as _;
use gdtf_assets::{
    ContentFileStem, ContentIntegrityReport, ContentSourcePaths, ContentValidationDone,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::level::PrefabRegistry;
use gdtf_content_families::{PrefabsFamily, prefabs::member_key};

use crate::{
    advance::advance_to_published, findings::has_dangling_ref, harness::editor_app_on_fixture_root,
};

const DANGLING_PLACEMENT: &str = "00000000-0000-0000-0000-063000000f01";

// The stem both fixture prefabs share, one under 3x3 and one under 4x4.
const SHARED_STEM: &str = "fixture_prefab";

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

#[test]
fn two_same_stem_prefabs_each_record_their_own_file() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let world = app.world();
    let registry = world.get_resource::<PrefabRegistry>();
    let sources = world.get_resource::<ContentSourcePaths<PrefabsFamily>>();
    assert!(
        registry.is_some() && sources.is_some(),
        "the prefab family must publish both its registry and its source paths",
    );
    let (Some(registry), Some(sources)) = (registry, sources) else {
        return;
    };

    let mut paths: Vec<String> = registry
        .iter()
        .filter(|prefab| **prefab.name() == *SHARED_STEM)
        .filter_map(|prefab| {
            let stem = ContentFileStem::new((**prefab.name()).clone());
            sources
                .path(&member_key(&stem, prefab.spec()))
                .map(|path| path.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    paths.sort();

    assert_eq!(
        paths,
        vec![
            "content/maps/fixture_theme/3x3/fixture_prefab.prefab.ron".to_owned(),
            "content/maps/fixture_theme/4x4/fixture_prefab.prefab.ron".to_owned(),
        ],
        "two prefabs sharing a file stem must each record their OWN file: the key a prefab is \
         recorded under names its whole registry key, so a stem-only key would collide in the \
         source-path map and lose one of them",
    );
}
