//! The delete registry the editor builds holds exactly the two families this build deletes.

use gdtf_content_editor::DeleteRegistry;

use crate::harness::{advance_to_published, editor_app_with_asset_root};

#[test]
fn the_editor_app_offers_a_delete_for_the_prefab_and_the_weighting_table_and_nothing_else() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);

    let mut labels: Vec<String> = app
        .world()
        .resource::<DeleteRegistry>()
        .labels()
        .map(|label| (**label).clone())
        .collect();
    labels.sort();

    assert_eq!(
        labels,
        vec!["InjuryTables".to_owned(), "PrefabRegistry".to_owned()],
        "this build deletes the prefab and the injury weighting table and nothing else; every \
         other content type belongs to a later child. Found: {labels:?}",
    );
}
