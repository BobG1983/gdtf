use super::{Prefab, PrefabKey, PrefabRegistry, PrefabSpec, SpawnRole, TerrainPlacementEntry};
use crate::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, PrefabName, ThemeUuid},
    metric::{Cell, CellLevel, Level},
    terrain::def::TerrainUuid,
};

fn theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00a1))
}

fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

#[test]
fn omitted_role_defaults_to_fill() {
    let ron = r#"(
        theme: "01840a3e-0000-4000-8000-0000000000a1",
        size: (width: 3, height: 3, levels: 1),
        placements: [
            (piece: "01840a3e-0000-4000-8000-0000000000b1", at: (cell: (x: 1, y: 1), level: 0)),
        ],
    )"#;
    let parsed = ron::de::from_str::<PrefabSpec>(ron);
    assert!(
        parsed.is_ok(),
        "a prefab RON literal omitting role must parse: {parsed:?}",
    );
    let Ok(spec) = parsed else { return };
    assert_eq!(
        spec.role,
        SpawnRole::Fill,
        "an omitted role must deserialize to the SpawnRole::Fill serde default",
    );
}

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
    let parsed = ron::de::from_str::<PrefabSpec>(ron);
    assert!(
        parsed.is_ok(),
        "a prefab RON literal with a single placements list must parse: {parsed:?}",
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
    let spec = PrefabSpec::new(theme, size, SpawnRole::Enemy, placements);

    let Ok(text) = ron::ser::to_string(&spec) else {
        return;
    };
    let reparsed = ron::de::from_str::<PrefabSpec>(&text);
    assert_eq!(
        reparsed.ok(),
        Some(spec),
        "a prefab spec must survive a serialize -> deserialize round-trip identically",
    );
}

#[test]
fn registry_inserts_and_retrieves_by_key() {
    let Some(size) = small_size() else { return };
    let theme = theme_uuid();
    let placements = vec![TerrainPlacementEntry::new(
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_00b1)),
        CellLevel::new(Cell::new(1, 1), Level::new(0)),
    )];
    let spec = PrefabSpec::new(theme, size, SpawnRole::Player, placements);
    let prefab = Prefab::new(PrefabName::new("entry_pad".to_owned()), spec);

    let mut registry = PrefabRegistry::default();
    registry.insert(prefab);

    let player_key = PrefabKey::new(theme, size, SpawnRole::Player);
    let enemy_key = PrefabKey::new(theme, size, SpawnRole::Enemy);

    assert_eq!(
        registry.prefabs_for(&player_key).len(),
        1,
        "the inserted Prefab is listed under its (theme, size, role) key",
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

#[test]
fn prefab_new_accepts_zero_placement_spec_infallibly() {
    let Some(size) = small_size() else { return };
    let spec = PrefabSpec::new(theme_uuid(), size, SpawnRole::Fill, Vec::new());
    assert!(
        spec.placements.is_empty(),
        "the fixture spec is openingless (zero placements)",
    );

    let prefab = Prefab::new(PrefabName::new("sealed_box".to_owned()), spec);
    assert!(
        prefab.spec().placements.is_empty(),
        "an openingless (zero-placement) spec yields a valid Prefab with no validation",
    );
    assert_eq!(
        **prefab.name(),
        "sealed_box".to_owned(),
        "the constructed prefab carries its name",
    );
}

#[test]
fn zero_placements_still_deserializes() {
    let ron = r#"(
        theme: "01840a3e-0000-4000-8000-0000000000a1",
        size: (width: 3, height: 3, levels: 1),
        placements: [],
    )"#;
    let parsed = ron::de::from_str::<PrefabSpec>(ron);
    assert!(
        parsed.is_ok(),
        "a prefab RON literal with zero placements must still parse: {parsed:?}",
    );
    let Ok(spec) = parsed else { return };
    assert!(
        spec.placements.is_empty(),
        "the empty placements list deserializes to an empty Vec",
    );
}
