use bevy::{MinimalPlugins, prelude::*};
use cobalt_test_utils::unwatched_asset_plugin;

use crate::{
    GdtfBattleInputPlugin,
    contextual::SlotRank,
    keybinds::table::{BoundKey, Keybinds},
};

const RON: &str = include_str!("../../../../../assets/core_tuning/keybinds.tuning.ron");

#[test]
fn shipped_keybinds_ron_matches_the_compiled_default() {
    let parsed: Result<Keybinds, _> = ron::from_str(RON);
    assert_eq!(
        parsed,
        Ok(Keybinds::default()),
        "the shipped keybinds.tuning.ron and `Keybinds::default()` are the same table — rebind a \
         key in one and it must move in the other; the file parsed to {parsed:?} and the compiled \
         default is {:?}",
        Keybinds::default(),
    );
}

#[test]
fn keybinds_exist_from_plugin_build_without_an_asset_server() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GdtfBattleInputPlugin);

    assert_eq!(
        app.world().get_resource::<Keybinds>(),
        Some(&Keybinds::default()),
        "with no AssetServer the hot-RON install returns early, so the table has to be inserted \
         by the registration itself",
    );
}

#[test]
fn keybinds_exist_from_plugin_build_with_an_asset_server() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(unwatched_asset_plugin());
    app.add_plugins(GdtfBattleInputPlugin);

    assert_eq!(
        app.world().get_resource::<Keybinds>(),
        Some(&Keybinds::default()),
        "the table is there before the first frame, so nothing has to wait for the asset",
    );
}

#[test]
fn shipped_keybinds_ron_deserializes_and_every_act_resolves() {
    let parsed: Result<Keybinds, _> = ron::from_str(RON);
    assert!(
        parsed.is_ok(),
        "the shipped keybinds.tuning.ron must deserialize into Keybinds: {:?}",
        parsed.err(),
    );
    let Ok(binds) = parsed else { return };

    let bound = [
        binds.select_clear(),
        binds.level_up(),
        binds.level_down(),
        binds.stance_cycle(),
        binds.aim_toggle(),
        binds.facing_cycle(),
    ];
    for (i, lhs) in bound.iter().enumerate() {
        for rhs in &bound[i + 1..] {
            assert_ne!(
                lhs, rhs,
                "no two bound acts may share a key (shipped keybinds.tuning.ron has a collision)",
            );
        }
    }

    assert_eq!(
        binds.select_next(),
        binds.select_prev(),
        "select_next / select_prev are one Shift-modified chord — they bind to the same key",
    );
    for other in &bound {
        assert_ne!(
            *other,
            binds.select_next(),
            "the cycle key must not collide with another bound act (shipped keybinds.tuning.ron)",
        );
    }
}

#[test]
fn bound_key_resolves_to_its_key_code() {
    assert_eq!(BoundKey::KeyEscape.key_code(), KeyCode::Escape);
    assert_eq!(BoundKey::KeyPageUp.key_code(), KeyCode::PageUp);
    assert_eq!(BoundKey::KeyPageDown.key_code(), KeyCode::PageDown);
    assert_eq!(BoundKey::KeyBracketLeft.key_code(), KeyCode::BracketLeft);
    assert_eq!(BoundKey::KeyBracketRight.key_code(), KeyCode::BracketRight);
    assert_eq!(BoundKey::KeyTab.key_code(), KeyCode::Tab);
    assert_eq!(BoundKey::KeyDigit1.key_code(), KeyCode::Digit1);
    assert_eq!(BoundKey::KeyDigit8.key_code(), KeyCode::Digit8);
    assert_eq!(BoundKey::KeyDigit9.key_code(), KeyCode::Digit9);
    assert_eq!(BoundKey::KeyEnter.key_code(), KeyCode::Enter);
    assert_eq!(BoundKey::KeyArrowUp.key_code(), KeyCode::ArrowUp);
    assert_eq!(BoundKey::KeyArrowDown.key_code(), KeyCode::ArrowDown);
    assert_eq!(BoundKey::KeyArrowLeft.key_code(), KeyCode::ArrowLeft);
    assert_eq!(BoundKey::KeyArrowRight.key_code(), KeyCode::ArrowRight);
}

#[test]
fn contextual_slot_key_maps_each_rank_to_its_digit() {
    assert_eq!(
        Keybinds::contextual_slot_key(SlotRank::new(1)),
        Some(KeyCode::Digit1),
        "visible slot rank 1 binds to Digit1",
    );
    assert_eq!(
        Keybinds::contextual_slot_key(SlotRank::new(8)),
        Some(KeyCode::Digit8),
        "the 8th visible slot (the current max act count) binds to Digit8",
    );
    assert_eq!(
        Keybinds::contextual_slot_key(SlotRank::new(9)),
        Some(KeyCode::Digit9),
        "the vocabulary has headroom to a 9th slot (Digit9)",
    );
    assert_eq!(
        Keybinds::contextual_slot_key(SlotRank::new(10)),
        None,
        "a rank past the nine-digit vocabulary binds to nothing (no crash, no wrap)",
    );
}
