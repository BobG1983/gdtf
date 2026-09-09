//! The weighting delete resolves its own file, and taking the table out re-arms validation.

use bevy::asset::Assets;
use cobalt_ron_assets::RonAsset;
use gdtf_assets::{ContentValidationDone, FindingFamily};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables, InjuryWeighting},
    severity::Severity,
};
use gdtf_content_families::injuries::weighting_member_key;
use gdtf_editor::{
    DeleteOutcome, DeleteRegistry, DeleteRequest, EditorMcpAssetsRoot, weighting_save_path_in,
};

use crate::{
    content_shared::{advance::advance_to_published, app::editor_app_with_asset_root},
    delete::{
        fixture::{WEIGHTED_INJURY, write_fixture_injury, write_fixture_weighting},
        harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
    },
};

/// The finding family label every injury weighting finding carries.
const WEIGHTING_FAMILY: &str = "InjuryTables";

/// The table this suite deletes.
const CATEGORY: InjuryCategory = InjuryCategory::Head;

/// The damage context half of that table's key.
const CONTEXT: DamageContext = DamageContext::Ranged;

#[test]
fn the_weighting_entry_resolves_the_file_its_own_save_path_names() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    let key = weighting_member_key(CATEGORY, CONTEXT);
    let world = app.world();
    let entry = world
        .resource::<DeleteRegistry>()
        .entry(&FindingFamily::new(WEIGHTING_FAMILY.to_owned()));
    assert!(
        entry.is_some(),
        "the editor app must register a delete entry for the injury weighting table",
    );
    let Some(entry) = entry else { return };
    let relative = entry.relative_path(world, &key);
    assert!(
        relative.is_some(),
        "the weighting entry resolves its file from the key alone, so it must answer one",
    );
    let Some(relative) = relative else { return };

    assert_eq!(
        dir.path().join(&*relative),
        weighting_save_path_in(dir.path(), CATEGORY, CONTEXT),
        "the delete must remove the file the weighting form's own save wrote, joined onto the \
         QA assets root — a resolver reading the wrong category or context names another table",
    );
}

#[test]
fn deleting_a_weighting_table_takes_its_buckets_out_and_re_arms_validation() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_fixture_injury(dir.path(), CATEGORY),
        "the fixture injury def write must succeed",
    );
    assert!(
        write_fixture_weighting(dir.path(), CATEGORY, CONTEXT),
        "the fixture weighting write must succeed",
    );
    let file = weighting_save_path_in(dir.path(), CATEGORY, CONTEXT);

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    assert!(
        app.world()
            .resource::<InjuryTables>()
            .table_for_category(CATEGORY, CONTEXT, Severity::Minor)
            .is_some(),
        "the authored weighting must have built a Minor bucket before the delete runs; the \
         injury `{WEIGHTED_INJURY}` is the row that resolves",
    );

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(WEIGHTING_FAMILY.to_owned()),
        weighting_member_key(CATEGORY, CONTEXT),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "taking the table out must re-arm validation so the delete settles within \
         {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "no record names a weighting table, so its in-use check finds nothing",
    );
    assert!(
        app.world()
            .resource::<InjuryTables>()
            .table_for_category(CATEGORY, CONTEXT, Severity::Minor)
            .is_none(),
        "the deleted table's buckets must stay out of InjuryTables",
    );
    assert!(
        app.world()
            .resource::<Assets<RonAsset<InjuryWeighting>>>()
            .iter()
            .all(|(_id, weighting)| weighting.category != CATEGORY || weighting.context != CONTEXT),
        "the authored asset must go with the buckets — a stale handle leaves the reference \
         check reading a table whose file and record are already gone",
    );
    assert!(
        !file.exists(),
        "the deleted table's file must be gone — otherwise the next reload brings it back",
    );
    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some(),
        "the delete settles on a republished report, so validation must have re-armed and run \
         again rather than stalling",
    );
}
