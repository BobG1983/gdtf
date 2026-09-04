//! A prefab no record names is removed from its registry and its file goes with it.

use gdtf_assets::{ContentFileStem, FindingFamily};
use gdtf_battle_sim::level::PrefabRegistry;
use gdtf_content_editor::{DeleteOutcome, DeleteRequest, EditorQaAssetsRoot};
use gdtf_content_families::prefabs::member_key;

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    fixture::{FIXTURE_PREFAB, prefab_spec, write_fixture_prefab},
    harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
};

/// The finding family label every prefab finding carries.
const PREFAB_FAMILY: &str = "PrefabRegistry";

#[test]
fn a_prefab_no_record_names_is_removed_with_its_file() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    let written = write_fixture_prefab(dir.path());
    assert!(
        written.is_some(),
        "the fixture prefab write must succeed under the temp assets root",
    );
    let Some(written) = written else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorQaAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    let key = member_key(
        &ContentFileStem::new(FIXTURE_PREFAB.to_owned()),
        &prefab_spec(),
    );
    assert!(
        app.world()
            .resource::<PrefabRegistry>()
            .iter()
            .any(|prefab| **prefab.name() == *FIXTURE_PREFAB),
        "the prefab folder must have resolved before the delete runs",
    );

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(PREFAB_FAMILY.to_owned()),
        key,
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );

    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "nothing names a prefab, so its in-use check finds nothing and the delete goes through",
    );
    assert!(
        !app.world()
            .resource::<PrefabRegistry>()
            .iter()
            .any(|prefab| **prefab.name() == *FIXTURE_PREFAB),
        "the removed prefab must stay out of PrefabRegistry",
    );
    assert!(
        !written.exists(),
        "the removed prefab's file must be gone — otherwise the next folder reload brings it \
         back",
    );
}
