//! A candidate row reads as its display name, and carries its key only where two names collide.

use gdtf_assets::ContentMemberKey;

use crate::delete::offer::{ReplacementLabel, labelled_candidates};

// One key and label pair for the row builder.
fn row(key: &str, label: &str) -> (ContentMemberKey, ReplacementLabel) {
    (
        ContentMemberKey::new(key.to_owned()),
        ReplacementLabel::new(label.to_owned()),
    )
}

// The label the builder answered for `key`.
fn label_for(rows: &[(ContentMemberKey, ReplacementLabel)], key: &str) -> String {
    labelled_candidates(rows.to_vec())
        .into_iter()
        .find(|candidate| (**candidate.key()).as_str() == key)
        .map_or_else(|| format!("no row for {key}"), |c| (**c.label()).clone())
}

#[test]
fn a_display_name_only_one_record_holds_is_the_whole_row() {
    let rows = vec![
        row("aaa", "Rusted Bulkhead"),
        row("bbb", "Rusted Bulkhead"),
        row("ccc", "Girder"),
    ];

    assert_eq!(
        label_for(&rows, "ccc"),
        "Girder".to_owned(),
        "a display name no other record shares reads on its own, with no key in brackets",
    );
}

#[test]
fn a_display_name_two_records_share_carries_each_records_key_in_brackets() {
    let rows = vec![
        row("aaa", "Rusted Bulkhead"),
        row("bbb", "Rusted Bulkhead"),
        row("ccc", "Girder"),
    ];

    assert_eq!(
        label_for(&rows, "aaa"),
        "Rusted Bulkhead  [aaa]".to_owned(),
        "a shared display name carries that record's key, so the author can tell the two rows \
         apart",
    );
    assert_eq!(
        label_for(&rows, "bbb"),
        "Rusted Bulkhead  [bbb]".to_owned(),
        "the second record sharing the name carries its own key, not the first record's",
    );
}
