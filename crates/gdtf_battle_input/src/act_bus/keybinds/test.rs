//! Tests for the keybind table + its loader (relocated from `keybinds.rs`, GTW-201).

use bevy::prelude::KeyCode;

use crate::keybinds::table::{BoundKey, Keybinds};

/// AC7 — the shipped keybind RON deserializes through the generic
/// `ron::from_str` path the loader uses, and every declared act name resolves
/// to a `KeyCode`. Parsing the embedded file contents proves the schema +
/// the file agree (a malformed or incomplete file would fail here): serde
/// rejects a missing field, so a successful parse proves all six bound acts
/// are present and each resolves through [`BoundKey::key_code`].
///
/// It does NOT pin the six authored `KeyCode` MAGNITUDES — `keybinds.tuning.ron` is
/// editable, hot-swappable tuning data ("edit freely — every binding is data"),
/// so locking the file's chosen keys would be a brittle test on editable data
/// (the metric-constant exemption does not apply to keybinds). The non-brittle
/// INVARIANT we do assert is that the six bound keys are mutually distinct —
/// no two acts share a key, whatever the author binds them to. (The
/// `fire_mode_cycle` binding was REMOVED in GTW-254 — the popup picker replaced
/// the blind cycle.)
#[test]
fn shipped_keybinds_ron_deserializes_and_every_act_resolves() {
    // The exact bytes the loose `assets/core_tuning/keybinds.tuning.ron` ships, parsed the
    // same way `RonAssetLoader` parses them (`ron::de::from_bytes`).
    const RON: &str = include_str!("../../../../../assets/core_tuning/keybinds.tuning.ron");
    let parsed: Result<Keybinds, _> = ron::from_str(RON);
    assert!(
        parsed.is_ok(),
        "the shipped keybinds.tuning.ron must deserialize into Keybinds: {:?}",
        parsed.err(),
    );
    let Ok(binds) = parsed else { return };

    // Non-brittle invariant: the distinctly-keyed bound acts are mutually distinct (no two
    // collide on the same key), independent of which keys the author chose. GTW-458's
    // `select_next` / `select_prev` are DELIBERATELY the same key (Tab), differentiated by the
    // held Shift modifier (Tab = Next, Shift+Tab = Prev — one physical chord), so they are
    // EXCLUDED from this mutual-distinctness set; the dedicated check below pins their shared
    // binding instead.
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

    // GTW-458 — the Prev/Next cycle is ONE chord: `select_next` and `select_prev` bind to the
    // SAME key, differentiated by Shift. The keyboard surface reads `select_next` + the Shift
    // modifier, so this shared binding is the authored intent (not a collision). It must ALSO
    // not collide with any other act key (else Tab would fire two acts at once).
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

/// Each `BoundKey` variant resolves to its documented `KeyCode` — the single
/// typed translation the systems rely on (no hardcoded literal elsewhere).
#[test]
fn bound_key_resolves_to_its_key_code() {
    assert_eq!(BoundKey::KeyEscape.key_code(), KeyCode::Escape);
    assert_eq!(BoundKey::KeyPageUp.key_code(), KeyCode::PageUp);
    assert_eq!(BoundKey::KeyPageDown.key_code(), KeyCode::PageDown);
    assert_eq!(BoundKey::KeyBracketLeft.key_code(), KeyCode::BracketLeft);
    assert_eq!(BoundKey::KeyBracketRight.key_code(), KeyCode::BracketRight);
    assert_eq!(BoundKey::KeyTab.key_code(), KeyCode::Tab);
}
