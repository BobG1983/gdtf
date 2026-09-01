use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabSpec, SpawnRole,
        ThemeUuid,
    },
    metric::{CellLevel, Level},
    prelude::Cell,
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};

use super::project::{editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab};
use crate::{EditorMap, session::MapEditorSession};

fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

const FLOOR: TerrainUuid = tu(0x0184_0a91_0004);
const WALL: TerrainUuid = tu(0x0184_0a91_0002);
const SLAB: TerrainUuid = tu(0x0184_0a91_0005);
const LADDER: TerrainUuid = tu(0x0184_0a91_00aa);

fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(2))
        .unwrap_or_else(|_| GridSize::default())
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn cover_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn wall_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(80),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags: Vec::new(),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn slab_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(50),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (FLOOR, cover_def(FLOOR, "Deck Floor")),
        (WALL, wall_def(WALL, "Bulkhead Wall")),
        (SLAB, slab_def(SLAB, "Deck Slab")),
        (LADDER, cover_def(LADDER, "Steel Ladder")),
    ])
}

fn session() -> MapEditorSession {
    MapEditorSession::new(theme(), Some(FLOOR), size())
}

#[test]
fn sanitize_name_folds_to_stem() {
    assert_eq!(sanitize_name("  Entry Room  ").as_str(), "entry_room");
    assert_eq!(sanitize_name("Sump-Waste 2!").as_str(), "sump_waste_2");
    assert!(
        sanitize_name("   ").is_empty(),
        "a blank name sanitizes to empty"
    );
}

#[test]
fn save_path_is_themed_sized_and_dot_prefab_ron() {
    use gdtf_assets::workspace_assets_root;
    use gdtf_content_families::prefabs::PREFABS_FOLDER;

    let Some(assets) = workspace_assets_root() else {
        unreachable!("this repo has a Cargo.lock above every crate");
    };
    let Some(path) = prefab_save_path("Industrial Hive", size(), "entry_room") else {
        unreachable!("the same search just answered above");
    };
    assert!(
        path.starts_with(assets.join(PREFABS_FOLDER)),
        "under the ONE shared assets root + prefab folder: {path:?}",
    );
    let tail: Vec<_> = path
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        tail[0], "entry_room.prefab.ron",
        "the .prefab.ron extension"
    );
    assert_eq!(tail[1], "4x4", "the <width>x<height> size dir");
    assert_eq!(
        tail[2], "industrial_hive",
        "the slugified theme display-name dir (not a closed-enum match)",
    );
}

#[test]
fn editor_map_round_trips_through_the_loader() {
    let reg = registry();
    let session = session();
    let mut map = EditorMap::new();
    let size = size();

    let north = TerrainFacing::default();
    assert!(map.paint(Cell::new(0, 1), FLOOR, north, size));
    assert!(map.paint(Cell::new(3, 3), WALL, TerrainFacing::East, size));
    assert!(map.paint(Cell::new(2, 2), SLAB, north, size));
    assert!(map.paint_at(
        CellLevel::new(Cell::new(1, 1), Level::new(1)),
        LADDER,
        north,
        size,
    ));

    let built = editor_map_to_prefab(&map, &reg, &session);
    assert!(
        built.is_ok(),
        "the map must project to a prefab: {:?}",
        built.as_ref().err(),
    );
    let Ok(saved) = built else { return };
    let serialized = serialize_prefab(&saved);
    assert!(
        serialized.is_ok(),
        "serializing the prefab must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded = ron::de::from_str::<PrefabSpec>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized prefab must round-trip through the PrefabSpec deserializer: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };

    let validated = Prefab::new(PrefabName::new("entry_room".to_owned()), reloaded.clone());
    assert_eq!(
        validated.spec(),
        &reloaded,
        "the built prefab carries the reloaded spec unchanged (no validation drops data)",
    );

    assert_eq!(
        reloaded, saved,
        "the reloaded prefab must equal the saved one — every cell, level, theme, and size \
         survives the round-trip (C2)",
    );
    assert_eq!(
        saved.placements.len(),
        4,
        "the four painted cells must each be one placement (C2)",
    );
    let east = reloaded
        .placements
        .iter()
        .filter(|entry| entry.facing == TerrainFacing::East)
        .count();
    assert_eq!(
        east, 1,
        "exactly the one cell painted East must come back East — the other three were \
         painted North; got {:?}",
        reloaded.placements,
    );
    assert_eq!(
        saved.role,
        SpawnRole::Fill,
        "a saved prefab is authored as the connective Fill default (C2)",
    );
    assert_eq!(
        reloaded.role,
        SpawnRole::Fill,
        "the connective Fill role survives the round-trip (C2)",
    );
}

#[test]
fn illegal_cell_rejects_the_save() {
    let reg = registry();
    let session = session();
    let mut illegal = EditorMap::new();
    let size = size();

    let north = TerrainFacing::default();
    assert!(illegal.paint_at(ground(1, 1), LADDER, north, size));
    assert!(illegal.paint_at(
        CellLevel::new(Cell::new(1, 1), Level::new(1)),
        SLAB,
        north,
        size,
    ));

    let built = editor_map_to_prefab(&illegal, &reg, &session);
    assert!(
        built.is_err(),
        "a map with a slab sealing a ladder must be REJECTED (C3) — got {built:?}",
    );
}
