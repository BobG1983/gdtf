use crate::mcp::{
    commands::read::families::sort::sorted_by_key,
    wire::{EditorFamilyEntryNet, EditorFamilyLabelNet, EditorKeyNet},
};

// Two entries this case owns, so it never depends on what a loaded registry happens to hold.
fn entry(key: &str) -> EditorFamilyEntryNet {
    EditorFamilyEntryNet::new(
        EditorKeyNet::new(key.to_owned()),
        EditorFamilyLabelNet::new(key.to_owned()),
    )
}

fn keys(entries: &[EditorFamilyEntryNet]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| (**entry.key()).clone())
        .collect()
}

#[test]
fn entries_come_back_in_key_order_whatever_order_they_went_in() {
    let sorted = sorted_by_key(vec![entry("mesh_armor"), entry("flak_vest")]);
    assert_eq!(
        keys(&sorted),
        vec!["flak_vest".to_owned(), "mesh_armor".to_owned()],
        "the input arrived in the reverse of key order, so an unsorted reply would hand it back \
         unchanged. A registry is a HashMap underneath, so its own order is not stable \
         enough for a client to assert on",
    );
}

#[test]
fn entries_already_in_key_order_are_left_alone() {
    let sorted = sorted_by_key(vec![entry("flak_vest"), entry("mesh_armor")]);
    assert_eq!(
        keys(&sorted),
        vec!["flak_vest".to_owned(), "mesh_armor".to_owned()],
        "sorting is the only reordering, so a list that is already in key order comes back as \
         it went in",
    );
}
