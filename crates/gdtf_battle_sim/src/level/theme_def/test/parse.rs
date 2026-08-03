use super::super::UuidThemeDef;

/// proving the [`TerrainUuid`](crate::terrain::def::TerrainUuid) `#[serde(transparent)]` wire
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
