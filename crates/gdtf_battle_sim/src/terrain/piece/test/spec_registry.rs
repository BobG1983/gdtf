//! Tests for the `TerrainSpec` authoring struct + the `TerrainRegistry` (GTW-394).
//! Mirrors the `weapon/test/spec_registry.rs` pattern — no magnitude assertions
//! (the authored numbers are tuning DATA, not pinned by tests; the brittle-test
//! rule / memory: *loader-tests-no-magnitude-pins*). Asserts only structure/variant
//! routing and key resolution.

use super::super::*;

// ── Compile-time path verification: the shipped asset files exist ────────────
// `include_str!` fails at compile time if the path does not resolve —
// a regression in an authored file immediately turns this red.
const SHIPPED_DECK_FLOOR_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/deck_floor.terrain.ron"
));

const SHIPPED_BULKHEAD_WALL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/terrain/bulkhead_wall.terrain.ron"
));

/// GTW-394 AC1 (parse + variant routing) — the shipped
/// `assets/content/terrain/deck_floor.terrain.ron` parses into a `TerrainSpec` with
/// `kind == Floor(_)`, and the shipped
/// `assets/content/terrain/bulkhead_wall.terrain.ron` parses as `Wall(_)`. No
/// magnitude assertions — structure/variant only.
#[test]
fn shipped_terrain_spec_parses_and_keys() {
    let floor_parsed = ron::de::from_str::<TerrainSpec>(SHIPPED_DECK_FLOOR_RON);
    assert!(
        floor_parsed.is_ok(),
        "the shipped assets/content/terrain/deck_floor.terrain.ron must parse into a TerrainSpec: \
         {floor_parsed:?}",
    );
    if let Ok(spec) = floor_parsed {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Floor(_)),
            "deck_floor.terrain.ron must parse as a Floor kind, got {:?}",
            spec.kind,
        );
    }

    let wall_parsed = ron::de::from_str::<TerrainSpec>(SHIPPED_BULKHEAD_WALL_RON);
    assert!(
        wall_parsed.is_ok(),
        "the shipped assets/content/terrain/bulkhead_wall.terrain.ron must parse into a TerrainSpec: \
         {wall_parsed:?}",
    );
    if let Ok(spec) = wall_parsed {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Wall(_)),
            "bulkhead_wall.terrain.ron must parse as a Wall kind, got {:?}",
            spec.kind,
        );
    }
}

/// GTW-394 AC1 (round-trip, all five variants) — inline RON with distinct arbitrary
/// literals for each of the five `TerrainKindSpec` variants parses to the right
/// variant. The spot values are fixtures only (mechanism, not a balance pin); no
/// magnitudes are asserted.
#[test]
fn terrain_spec_round_trips_each_kind() {
    // Floor variant — carries a move cost, no HP.
    let floor_ron = r#"(
        graphic: "floor",
        footfall: "footfall_metal",
        kind: Floor((
            move_cost: 4,
        )),
    )"#;
    let floor = ron::de::from_str::<TerrainSpec>(floor_ron);
    assert!(floor.is_ok(), "Floor TerrainSpec must parse: {floor:?}");
    if let Ok(spec) = floor {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Floor(FloorSpec { .. })),
            "Floor kind must carry a FloorSpec, got {:?}",
            spec.kind,
        );
    }

    // Wall variant — StructuralSpec with a height band.
    let wall_ron = r#"(
        graphic: "wall",
        footfall: "footfall_metal",
        kind: Wall((
            max_hp: 40,
            armor_protection: 6,
            armor_hardness: 3,
            height_band: High,
        )),
    )"#;
    let wall = ron::de::from_str::<TerrainSpec>(wall_ron);
    assert!(wall.is_ok(), "Wall TerrainSpec must parse: {wall:?}");
    if let Ok(spec) = wall {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Wall(StructuralSpec { .. })),
            "Wall kind must carry a StructuralSpec, got {:?}",
            spec.kind,
        );
    }

    // Cover variant — StructuralSpec with a height band.
    let cover_ron = r#"(
        graphic: "cover",
        footfall: "footfall_metal",
        kind: Cover((
            max_hp: 20,
            armor_protection: 3,
            armor_hardness: 1,
            height_band: Low,
        )),
    )"#;
    let cover = ron::de::from_str::<TerrainSpec>(cover_ron);
    assert!(cover.is_ok(), "Cover TerrainSpec must parse: {cover:?}");
    if let Ok(spec) = cover {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Cover(StructuralSpec { .. })),
            "Cover kind must carry a StructuralSpec, got {:?}",
            spec.kind,
        );
    }

    // Scatter variant — StructuralSpec with a height band.
    let scatter_ron = r#"(
        graphic: "rubble",
        footfall: "footfall_rubble",
        kind: Scatter((
            max_hp: 8,
            armor_protection: 1,
            armor_hardness: 0,
            height_band: Low,
        )),
    )"#;
    let scatter = ron::de::from_str::<TerrainSpec>(scatter_ron);
    assert!(
        scatter.is_ok(),
        "Scatter TerrainSpec must parse: {scatter:?}"
    );
    if let Ok(spec) = scatter {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Scatter(StructuralSpec { .. })),
            "Scatter kind must carry a StructuralSpec, got {:?}",
            spec.kind,
        );
    }

    // Slab variant — SlabPieceSpec, NO height band.
    let slab_ron = r#"(
        graphic: "slab",
        footfall: "footfall_metal",
        kind: Slab((
            max_hp: 30,
            armor_protection: 5,
            armor_hardness: 2,
        )),
    )"#;
    let slab = ron::de::from_str::<TerrainSpec>(slab_ron);
    assert!(slab.is_ok(), "Slab TerrainSpec must parse: {slab:?}");
    if let Ok(spec) = slab {
        assert!(
            matches!(spec.kind, TerrainKindSpec::Slab(SlabPieceSpec { .. })),
            "Slab kind must carry a SlabPieceSpec (no height band), got {:?}",
            spec.kind,
        );
    }
}

/// GTW-394 AC2 (registry key + resolve) — a `TerrainRegistry` keys specs by
/// `TerrainName` and resolves a lookup: a present key returns the spec, an absent
/// key returns `None`. Built directly from `TerrainRegistry::new` (no `AssetServer`
/// — the sim-unit shape). No magnitude assertions — key routing only.
#[test]
fn terrain_registry_keys_and_resolves_by_name() {
    // Parse an inline Floor spec as the fixture — arbitrary distinct literals.
    let floor_ron = r#"(
        graphic: "floor",
        footfall: "footfall_metal",
        kind: Floor((
            move_cost: 4,
        )),
    )"#;
    let Ok(spec) = ron::de::from_str::<TerrainSpec>(floor_ron) else {
        return;
    };

    let deck_floor = TerrainName::new("deck_floor".to_owned());
    let registry = TerrainRegistry::new([(deck_floor.clone(), spec)]);

    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted piece"
    );
    assert!(!registry.is_empty(), "a one-piece registry is non-empty");
    assert!(
        registry.spec(&deck_floor).is_some(),
        "a present key resolves to its spec",
    );
    assert!(
        registry
            .spec(&TerrainName::new("missing".to_owned()))
            .is_none(),
        "an absent key resolves to None",
    );
}
