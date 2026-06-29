//! C2 — the SIM-OWNED `tags` vec defaults to EMPTY when omitted (serde default),
//! round-trips when present, and lives on the sim side of [`TerrainDef`] (reachable
//! from the sim crate), NOT on [`TerrainPresenterKind`].

use super::super::{TerrainDef, TerrainTag};

/// C2 — an omitted `tags` field parses as the EMPTY vec (`#[serde(default)]`).
#[test]
fn tags_default_empty_when_omitted() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000030",
        display_name: "Bulkhead Wall",
        sim_kind: Wall(hp: 40, armor_protection: 6, armor_hardness: 3, height_band: High),
        presenter_kind: Wall(graphic_name: "wall"),
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

/// C2 — a present `tags` list parses into the sim-owned vec, in order, with the
/// closed-enum variants. (The field is `def.tags`, read straight from the sim crate —
/// proving it is on the SIM side of `TerrainDef`, not on `TerrainPresenterKind`.)
#[test]
fn tags_round_trip_when_present() {
    let ron = r#"(
        key: "01840a3e-0000-4000-8000-000000000031",
        display_name: "Blast Door",
        sim_kind: Wall(hp: 80, armor_protection: 10, armor_hardness: 6, height_band: High),
        presenter_kind: Wall(graphic_name: "door"),
        tags: [Openable, BlocksVision, BlocksPathfinding, Indestructible],
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
                TerrainTag::BlocksVision,
                TerrainTag::BlocksPathfinding,
                TerrainTag::Indestructible,
            ],
            "the sim-owned tags must round-trip in order with the closed-enum variants",
        );
    }
}
