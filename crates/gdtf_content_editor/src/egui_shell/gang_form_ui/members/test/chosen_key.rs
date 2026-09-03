//! Unit tests for the member loadout combos.

use super::super::{KeyChoice, chosen_key};

#[test]
fn the_clear_choice_maps_to_no_key() {
    assert_eq!(
        chosen_key(KeyChoice::Clear),
        None,
        "picking the clear row leaves the member holding no key",
    );
}

#[test]
fn a_key_choice_maps_to_that_key() {
    assert_eq!(
        chosen_key(KeyChoice::Key("flak_vest".to_owned())),
        Some("flak_vest".to_owned()),
        "picking a key row leaves the member holding that key",
    );
}
