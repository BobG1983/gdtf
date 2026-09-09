//! The replacement offer holds the family's other records, and not the one being deleted.

use cobalt_test_utils::advance_until;
use gdtf_editor::ReplacementOffer;

use crate::delete::{
    harness::{TERRAIN_FAMILY, editor_on, is_published},
    records::{
        DELETED_PIECE, DELETED_THEME, REPLACEMENT_PIECE, SPARE_PIECE, terrain_display_name,
        write_terrain_def, write_theme_def,
    },
};

#[test]
fn the_offer_holds_every_other_record_of_the_family_and_not_the_deleted_one() {
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    if write_terrain_def(dir.path(), "deleted_piece", DELETED_PIECE).is_none()
        || write_terrain_def(dir.path(), "replacement_piece", REPLACEMENT_PIECE).is_none()
        || write_terrain_def(dir.path(), "spare_piece", SPARE_PIECE).is_none()
        || write_theme_def(
            dir.path(),
            "floor_theme",
            DELETED_THEME,
            DELETED_PIECE,
            &[REPLACEMENT_PIECE],
        )
        .is_none()
    {
        return;
    }

    let mut app = editor_on(dir.path());
    app.insert_resource(gdtf_editor::DeleteRequest::new(
        gdtf_assets::FindingFamily::new(TERRAIN_FAMILY.to_owned()),
        gdtf_assets::ContentMemberKey::new(DELETED_PIECE.to_owned()),
    ));

    advance_until(&mut app, |app| {
        app.world().contains_resource::<ReplacementOffer>()
    });
    let mut rows: Vec<(String, String)> = app
        .world()
        .resource::<ReplacementOffer>()
        .candidates()
        .iter()
        .map(|candidate| ((**candidate.key()).clone(), (**candidate.label()).clone()))
        .collect();
    rows.sort();
    let keys: Vec<String> = rows.iter().map(|(key, _label)| key.clone()).collect();

    assert_eq!(
        keys,
        vec![REPLACEMENT_PIECE.to_owned(), SPARE_PIECE.to_owned()],
        "the offer names the registry as it stood before the record came out of it, minus the \
         record being deleted; published at the end: {}",
        is_published(&app),
    );
    let labels: Vec<String> = rows.iter().map(|(_key, label)| label.clone()).collect();
    assert_eq!(
        labels,
        vec![
            terrain_display_name("replacement_piece"),
            terrain_display_name("spare_piece"),
        ],
        "a candidate reads as its def's display name, not its key; every def under this root \
         carries a name of its own, so no row carries a key in brackets",
    );
}
