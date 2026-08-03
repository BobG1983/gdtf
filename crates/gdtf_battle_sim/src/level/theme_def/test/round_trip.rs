use super::super::{ThemeDisplayName, ThemeUuid, UuidThemeDef};
use crate::terrain::def::TerrainUuid;

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
