use super::super::{TerrainDef, TerrainTag};

/// C2 — an omitted `tags` field parses as the EMPTY vec (`#[serde(default)]`).
#[test]
fn tags_default_empty_when_omitted() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000030",
        display_name: "Bulkhead Wall",
        sim_kind: Wall(hp: 40, armor_protection: 6, armor_hardness: 3, height_band: High),
        presenter_kind: Wall(graphic_name: "wall"),
        views: [
            (view: Edge(North), sprite: "wall"),
            (view: Edge(East), sprite: "wall"),
            (view: Edge(South), sprite: "wall"),
            (view: Edge(West), sprite: "wall"),
            (view: Corner(NorthEast), sprite: "wall"),
            (view: Corner(SouthEast), sprite: "wall"),
            (view: Corner(SouthWest), sprite: "wall"),
            (view: Corner(NorthWest), sprite: "wall"),
        ],
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "TerrainDef without tags must parse: {parsed:?}"
    );
    if let Ok(def) = parsed {
        assert!(
            def.tags.is_empty(),
            "an omitted `tags` field must default to the empty vec, got {:?}",
            def.tags,
        );
    }
}

#[test]
fn tags_round_trip_when_present() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000031",
        display_name: "Blast Door",
        sim_kind: Wall(hp: 80, armor_protection: 10, armor_hardness: 6, height_band: High),
        presenter_kind: Wall(graphic_name: "door"),
        views: [
            (view: Shut(North), sprite: "door"),
            (view: Open(North), sprite: "door"),
            (view: Shut(East), sprite: "door"),
            (view: Open(East), sprite: "door"),
            (view: Shut(South), sprite: "door"),
            (view: Open(South), sprite: "door"),
            (view: Shut(West), sprite: "door"),
            (view: Open(West), sprite: "door"),
        ],
        tags: [Openable, Stair, BlocksVision, BlocksPathfinding, Indestructible],
    )"#;
    let parsed = ron::de::from_str::<TerrainDef>(ron);
    assert!(
        parsed.is_ok(),
        "TerrainDef with tags must parse: {parsed:?}"
    );
    if let Ok(def) = parsed {
        assert_eq!(
            def.tags,
            vec![
                TerrainTag::Openable,
                TerrainTag::Stair,
                TerrainTag::BlocksVision,
                TerrainTag::BlocksPathfinding,
                TerrainTag::Indestructible,
            ],
            "the sim-owned tags must round-trip in order with the closed-enum variants",
        );
    }
}
