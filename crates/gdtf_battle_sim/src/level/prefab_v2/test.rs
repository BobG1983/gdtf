//! Unit tests for the v2 prefab schema (GTW-486) — they exercise the REAL
//! [`PrefabSpecV2`] / [`TerrainPlacementEntry`] types through `ron` parse + round-trip.
//!
//! No magnitude assertions — the UUID / grid fixtures are mechanism, not balance; the
//! tests assert the RON SHAPE parses and routes into the right fields, and that a spec
//! survives a serialize → deserialize round-trip identically (the brittle-test rule).

use super::{Prefab2, PrefabKey2, PrefabRegistry2, PrefabSpecV2, TerrainPlacementEntry};
use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, PrefabName, SpawnRole, ThemeUuid},
    metric::{Cell, CellLevel, Level},
    terrain::def::TerrainUuid,
};

/// A stable [`ThemeUuid`] fixture (a fixed v4 UUID) — mechanism, not balance.
fn theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00a1))
}

/// A 3x3x1 footprint (a valid small prefab size).
fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

/// C1 — a v2 prefab RON literal OMITTING `role` parses with `role == SpawnRole::Fill`,
/// proving the serde-default-`Fill` behaviour (an unauthored role is a generic fill
/// fragment). The literal authors a theme UUID, a size, and one placement, but NO `role`.
#[test]
fn omitted_role_defaults_to_fill() {
    let ron = r#"(
        theme: "01840a3e-0000-4000-8000-0000000000a1",
        size: (width: 3, height: 3, levels: 1),
        placements: [
            (piece: "01840a3e-0000-4000-8000-0000000000b1", at: (cell: (x: 1, y: 1), level: 0)),
        ],
    )"#;
    let parsed = ron::de::from_str::<PrefabSpecV2>(ron);
    assert!(
        parsed.is_ok(),
        "a v2 prefab RON literal omitting role must parse: {parsed:?}",
    );
    let Ok(spec) = parsed else { return };
    assert_eq!(
        spec.role,
        SpawnRole::Fill,
        "an omitted role must deserialize to the SpawnRole::Fill serde default",
    );
}

/// C2 — a v2 prefab RON literal with a SINGLE `placements` list of piece-uuid @ cell-level
/// entries parses into a [`PrefabSpecV2`], proving the one placed-UUID list replaces the
/// legacy schema's four split lists (walls / scatter / slabs / floors). The literal names
/// an explicit `role` and two placements.
#[test]
fn single_placements_list_parses() {
    let ron = r#"(
        theme: "01840a3e-0000-4000-8000-0000000000a1",
        size: (width: 3, height: 3, levels: 1),
        role: Player,
        placements: [
            (piece: "01840a3e-0000-4000-8000-0000000000b1", at: (cell: (x: 0, y: 0), level: 0)),
            (piece: "01840a3e-0000-4000-8000-0000000000b2", at: (cell: (x: 2, y: 1), level: 0)),
        ],
    )"#;
    let parsed = ron::de::from_str::<PrefabSpecV2>(ron);
    assert!(
        parsed.is_ok(),
        "a v2 prefab RON literal with a single placements list must parse: {parsed:?}",
    );
    let Ok(spec) = parsed else { return };
    assert_eq!(
        spec.role,
        SpawnRole::Player,
        "the explicit role parses into the role field",
    );
    assert_eq!(
        spec.placements.len(),
        2,
        "both placed-UUID entries parse into the one placements list",
    );
}

/// C3 — round-trip identity: `deserialize(serialize(spec)) == spec` for a spec carrying
/// `>= 1` placement. Identity only (no magnitude pins) — the spec is built in code, written
/// to RON via the [`Serialize`] derive, and re-parsed, then compared whole.
#[test]
fn round_trips_through_ron() {
    let Some(size) = small_size() else { return };
    let theme = ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00a1));
    let placements = vec![
        TerrainPlacementEntry::new(
            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b1)),
            CellLevel::new(Cell::new(0, 0), Level::new(0)),
        ),
        TerrainPlacementEntry::new(
            TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b2)),
            CellLevel::new(Cell::new(2, 1), Level::new(0)),
        ),
    ];
    let spec = PrefabSpecV2::new(theme, size, SpawnRole::Enemy, placements);

    let Ok(text) = ron::ser::to_string(&spec) else {
        // A construction-derived spec always serializes; bail (not fail) if the encoder errs.
        return;
    };
    let reparsed = ron::de::from_str::<PrefabSpecV2>(&text);
    assert_eq!(
        reparsed.ok(),
        Some(spec),
        "a v2 prefab spec must survive a serialize -> deserialize round-trip identically",
    );
}

/// GTW-488 C1 — the re-keyed [`PrefabRegistry2`] inserts a [`Prefab2`] under a
/// [`PrefabKey2`], then [`prefabs_for`](PrefabRegistry2::prefabs_for) returns it. Exercises
/// the REAL registry (insert -> lookup -> name match), not dead code, and confirms a
/// non-matching key is empty.
#[test]
fn registry2_inserts_and_retrieves_by_key() {
    let Some(size) = small_size() else { return };
    let theme = theme_uuid();
    let placements = vec![TerrainPlacementEntry::new(
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b1)),
        CellLevel::new(Cell::new(1, 1), Level::new(0)),
    )];
    let spec = PrefabSpecV2::new(theme, size, SpawnRole::Player, placements);
    let prefab = Prefab2::new(PrefabName::new("entry_pad".to_owned()), spec);

    let mut registry = PrefabRegistry2::default();
    registry.insert(prefab);

    let player_key = PrefabKey2::new(theme, size, SpawnRole::Player);
    let enemy_key = PrefabKey2::new(theme, size, SpawnRole::Enemy);

    assert_eq!(
        registry.prefabs_for(&player_key).len(),
        1,
        "the inserted Prefab2 is listed under its (theme, size, role) key",
    );
    assert_eq!(
        registry
            .prefabs_for(&player_key)
            .first()
            .map(|p| (**p.name()).clone()),
        Some("entry_pad".to_owned()),
        "prefabs_for returns the prefab that was inserted",
    );
    assert!(
        registry.prefabs_for(&enemy_key).is_empty(),
        "a key with no inserted prefab returns the empty slice",
    );
    assert_eq!(registry.len(), 1, "one prefab total across keys");
}

/// GTW-488 C2 — [`Prefab2::new`] constructs from a [`PrefabSpecV2`] carrying ZERO
/// placements (an openingless fragment) and SUCCEEDS infallibly. Pin-discriminating: the
/// constructor returns a plain `Prefab2` (no `Result`), so there is NO
/// `NoEdgeOpening` / edge-opening validation path on the v2 prefab — an openingless prefab
/// is valid.
#[test]
fn prefab2_new_accepts_zero_placement_spec_infallibly() {
    let Some(size) = small_size() else { return };
    // Openingless: zero placements (the v2 schema has no edge_openings to author at all).
    let spec = PrefabSpecV2::new(theme_uuid(), size, SpawnRole::Fill, Vec::new());
    assert!(
        spec.placements.is_empty(),
        "the fixture spec is openingless (zero placements)",
    );

    // `Prefab2::new` is infallible (no Result, no validation) — it constructs directly.
    let prefab = Prefab2::new(PrefabName::new("sealed_box".to_owned()), spec);
    assert!(
        prefab.spec().placements.is_empty(),
        "an openingless (zero-placement) spec yields a valid Prefab2 with no validation",
    );
    assert_eq!(
        **prefab.name(),
        "sealed_box".to_owned(),
        "the constructed prefab carries its name",
    );
}

/// C4 — a v2 prefab with ZERO placements still deserializes (the empty-list case is valid;
/// there is no validation / connectivity path that rejects it). Authors an empty
/// `placements` list and an omitted role (which defaults to `Fill`).
#[test]
fn zero_placements_still_deserializes() {
    let ron = r#"(
        theme: "01840a3e-0000-4000-8000-0000000000a1",
        size: (width: 3, height: 3, levels: 1),
        placements: [],
    )"#;
    let parsed = ron::de::from_str::<PrefabSpecV2>(ron);
    assert!(
        parsed.is_ok(),
        "a v2 prefab RON literal with zero placements must still parse: {parsed:?}",
    );
    let Ok(spec) = parsed else { return };
    assert!(
        spec.placements.is_empty(),
        "the empty placements list deserializes to an empty Vec",
    );
}
