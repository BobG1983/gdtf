//! The delete registry the editor builds holds exactly the families this build deletes.

use gdtf_content_editor::DeleteRegistry;

use crate::harness::{advance_to_published, editor_app_with_asset_root};

#[test]
fn the_editor_app_offers_a_delete_for_every_family_this_build_deletes_and_nothing_else() {
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
        vec![
            "ArmorRegistry".to_owned(),
            "AttachmentRegistry".to_owned(),
            "FieldDefRegistry".to_owned(),
            "InjuryRegistry".to_owned(),
            "InjuryTables".to_owned(),
            "MeleeWeaponRegistry".to_owned(),
            "PrefabRegistry".to_owned(),
        ],
        "this build deletes the prefab, the injury weighting table, and the five families whose \
         reference is optional; terrain, theme, gang and weapon belong to a later child. Found: \
         {labels:?}",
    );
}
