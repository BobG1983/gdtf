//! C2 — round-trip IDENTITY: `deserialize(serialize(theme_def)) == theme_def`. NO
//! magnitude pins — identity only.

use super::super::{ThemeDisplayName, ThemeUuid, UuidThemeDef};
use crate::terrain::def::TerrainUuid;

/// Serialize `def` to RON and parse it back, asserting the round-trip is the identity.
/// Returns silently (no panic) if either serde step fails, after asserting it succeeded —
/// the no-`unwrap`/`expect` house style.
fn assert_round_trips(def: &UuidThemeDef) {
    let serialized = ron::ser::to_string(def);
    assert!(
        serialized.is_ok(),
        "a UuidThemeDef must serialize to RON: {serialized:?}",
    );
    let Ok(text) = serialized else { return };

    let reparsed = ron::de::from_str::<UuidThemeDef>(&text);
    assert!(
        reparsed.is_ok(),
        "the serialized RON must parse back into a UuidThemeDef: {reparsed:?} (from {text})",
    );
    let Ok(round_tripped) = reparsed else { return };

    assert_eq!(
        &round_tripped, def,
        "deserialize(serialize(theme_def)) must equal theme_def (round-trip identity)",
    );
}

/// C2 — a theme definition with a non-empty terrain palette round-trips to itself
/// (identity), exercising the key UUID, the display name, the default-floor terrain UUID,
/// and the terrain UUID list through the round-trip.
#[test]
fn theme_def_round_trips() {
    let floor = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b1));
    let def = UuidThemeDef {
        key:           ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00a1)),
        display_name:  ThemeDisplayName::new("Industrial Hive".to_owned()),
        default_floor: floor,
        terrain:       vec![
            floor,
            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b2)),
            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b3)),
        ],
    };
    assert_round_trips(&def);
}

/// C2 — a theme definition with an EMPTY terrain palette round-trips to itself (identity),
/// exercising the empty-vec path.
#[test]
fn empty_palette_theme_def_round_trips() {
    let def = UuidThemeDef {
        key:           ThemeUuid::generate(),
        display_name:  ThemeDisplayName::new("Sump Waste".to_owned()),
        default_floor: TerrainUuid::generate(),
        terrain:       Vec::new(),
    };
    assert_round_trips(&def);
}
