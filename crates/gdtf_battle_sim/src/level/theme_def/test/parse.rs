//! C1 — a theme RON literal (key uuid, `display_name`, `default_floor` terrain-uuid,
//! terrain list of uuids) parses into a [`UuidThemeDef`] via [`ron::de::from_str`].
//!
//! No magnitude assertions — the UUID fixtures are mechanism, not balance; this test
//! asserts the RON SHAPE parses and routes into the right fields (the brittle-test rule).

use super::super::UuidThemeDef;

/// C1 — a NAMED-STRUCT RON literal for a theme parses into a [`UuidThemeDef`]. The
/// `default_floor` is a single terrain UUID string and `terrain` is a list of UUID strings,
/// proving the [`TerrainUuid`](crate::terrain::def::TerrainUuid) `#[serde(transparent)]` wire
/// form rides correctly inside the theme.
#[test]
fn theme_def_ron_parses() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-0000000000a1",
        display_name: "Industrial Hive",
        default_floor: "01840a3e-0000-4000-8000-0000000000b1",
        terrain: [
            "01840a3e-0000-4000-8000-0000000000b1",
            "01840a3e-0000-4000-8000-0000000000b2",
            "01840a3e-0000-4000-8000-0000000000b3",
        ],
    )"#;
    let parsed = ron::de::from_str::<UuidThemeDef>(ron);
    assert!(parsed.is_ok(), "a theme RON literal must parse: {parsed:?}");
    let Ok(def) = parsed else { return };

    assert_eq!(
        &**def.display_name, "Industrial Hive",
        "the display name parses into the ThemeDisplayName field",
    );
    assert_eq!(
        def.terrain.len(),
        3,
        "the terrain palette parses its three UUID entries",
    );
    assert!(
        def.terrain.contains(&def.default_floor),
        "the default_floor UUID is one this theme's authored palette references",
    );
}
