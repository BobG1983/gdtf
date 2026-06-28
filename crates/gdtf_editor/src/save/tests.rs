//! In-crate tests for the GTW-432 save path — sanitize, path resolution, round-trip, and the
//! illegal-cell guard (C3 / C6 / C2 / C4 contract clauses).

use gdtf_battle_sim::{
    Cell,
    armor::{ArmorHardness, ArmorProtection},
    level::{
        CatalogTile, CatalogTileKind, GridHeight, GridLevels, GridSize, GridWidth, LevelTheme,
        Prefab, PrefabName, PrefabSpec, ThemeCatalogRegistry, ThemeSpec, ThemeTileCatalog,
        TileAtlasIndex, TileDisplayName, TileKey,
    },
    metric::{CellLevel, Level},
    slab::SlabHp,
    tuning::MoveCost,
};

use super::project::{editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab};
use crate::{EditorMap, session::MapEditorSession};

/// The theme the test catalog is keyed by.
const THEME: LevelTheme = LevelTheme::IndustrialHive;

/// A `4 × 4 × 2` drawable volume — wide enough for a boundary + a multi-level ladder, with a
/// `1 × 1 × 1` fallback (the constructor is fallible; the fallback keeps the test panic-free).
fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(2))
        .unwrap_or_else(|_| GridSize::default())
}

/// A catalog tile key.
fn key(id: &str) -> TileKey {
    TileKey::new(id.to_owned())
}

/// A ground `(cell, L0)` slot.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A FLOOR catalog tile (the vertical rules never constrain it).
fn floor_tile(label: &str) -> CatalogTile {
    CatalogTile {
        display_name: TileDisplayName::new(label.to_owned()),
        atlas_index:  TileAtlasIndex::new(6),
        kind:         CatalogTileKind::Floor {
            move_cost: MoveCost::new(4),
        },
    }
}

/// A SLAB catalog tile.
fn slab_tile(label: &str) -> CatalogTile {
    CatalogTile {
        display_name: TileDisplayName::new(label.to_owned()),
        atlas_index:  TileAtlasIndex::new(22),
        kind:         CatalogTileKind::Slab {
            max_hp:           SlabHp::new(50),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
    }
}

/// A registry with a default floor (`deck_floor`), a slab (`deck_slab`), and a ladder-named
/// tile (`steel_ladder`, recognised by name — the catalog has no ladder kind).
fn registry() -> ThemeCatalogRegistry {
    let mut tiles = bevy::platform::collections::HashMap::new();
    tiles.insert(key("deck_floor"), floor_tile("Deck Floor"));
    tiles.insert(key("deck_slab"), slab_tile("Deck Slab"));
    tiles.insert(key("steel_ladder"), floor_tile("Steel Ladder"));
    let spec = ThemeSpec {
        theme: THEME,
        default_floor: key("deck_floor"),
        tiles,
    };
    ThemeCatalogRegistry::new([(THEME, ThemeTileCatalog::from_spec(spec))])
}

/// A session at the test theme/size with `deck_floor` as the resolved default floor.
fn session() -> MapEditorSession {
    MapEditorSession::new(THEME, Some(key("deck_floor")), size())
}

/// `sanitize_name` folds a free-form name to the file-stem convention (trim, lowercase,
/// spaces → underscores, drop punctuation), and rejects a name that sanitizes to nothing.
#[test]
fn sanitize_name_folds_to_stem() {
    assert_eq!(sanitize_name("  Entry Room  "), "entry_room");
    assert_eq!(sanitize_name("Sump-Waste 2!"), "sump_waste_2");
    assert!(
        sanitize_name("   ").is_empty(),
        "a blank name sanitizes to empty"
    );
}

/// The save path is `assets/content/maps/<theme>/<size>/<stem>.prefab.ron` — the layout the
/// GTW-418 loader scans (the `.prefab` infix is required so the loader keys it).
#[test]
fn save_path_is_themed_sized_and_dot_prefab_ron() {
    let path = prefab_save_path(THEME, size(), "entry_room");
    let tail: Vec<_> = path
        .components()
        .rev()
        .take(4)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    // Reversed: file, size dir, theme dir, maps dir.
    assert_eq!(
        tail[0], "entry_room.prefab.ron",
        "the .prefab.ron extension"
    );
    assert_eq!(tail[1], "4x4", "the <width>x<height> size dir");
    assert_eq!(tail[2], "industrial_hive", "the snake_case theme dir");
    assert_eq!(tail[3], "maps", "under content/maps");
}

/// GTW-432 C4 — the IDENTITY round-trip on the REAL code path: build an [`EditorMap`] with a
/// boundary floor, a wall, a slab, AND a MULTI-LEVEL ladder cell, project it via the real
/// [`editor_map_to_prefab`] + [`serialize_prefab`], deserialize via the SAME parser the GTW-418
/// loader uses (`ron::de::from_str::<PrefabSpec>`), validate via the SAME [`Prefab::new`] the
/// loader runs, and assert the reloaded spec EQUALS the saved one — every cell, level, theme,
/// and size survives.
///
/// Pin-discriminating: if a field were dropped (a level, a cell, theme, or size) the reloaded
/// spec would differ from the saved one and the `assert_eq!` would fail. The multi-level ladder
/// + its derived vertical link pin the level dimension specifically.
///
/// In-memory string round-trip — it NEVER writes into the real `assets/` tree (no repo
/// pollution, deterministic). Uses `assert!` + `let … else { return }`, NOT `panic!`/`unwrap`
/// (denied lints even in tests under the strict workspace).
#[test]
fn editor_map_round_trips_through_the_loader() {
    let reg = registry();
    let session = session();
    let mut map = EditorMap::new();
    let size = size();

    // A walkable boundary floor cell (the derived edge-opening seam + a per-cell override).
    assert!(map.paint(Cell::new(0, 1), key("deck_floor"), size));
    // A wall on the back boundary corner.
    assert!(map.paint(Cell::new(3, 3), key("bulkhead_wall"), size));
    // A slab at an interior ground cell (multi-level geometry).
    assert!(map.paint(Cell::new(2, 2), key("deck_slab"), size));
    // A MULTI-LEVEL cell: a ladder at L0 (rises to L1) — pins the level dimension.
    assert!(map.paint(Cell::new(1, 1), key("steel_ladder"), size));

    // SAVE via the real projection + serialize.
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

    // RELOAD via the loader path: deserialize with the SAME parser RonAsset<PrefabSpec> uses …
    let reloaded = ron::de::from_str::<PrefabSpec>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized prefab must round-trip through the PrefabSpec deserializer: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };

    // … and VALIDATE via the SAME Prefab::new the loader runs (C6 edge-opening check) — so the
    // round-trip exercises the loader's real acceptance gate, not just the parser.
    let validated = Prefab::new(PrefabName::new("entry_room".to_owned()), reloaded.clone());
    assert!(
        validated.is_ok(),
        "the reloaded prefab must pass the loader's C6 validation: {:?}",
        validated.as_ref().err(),
    );

    // load(save(grid)) == grid: the reloaded spec equals the saved one (PrefabSpec: PartialEq).
    assert_eq!(
        reloaded, saved,
        "the reloaded prefab must equal the saved one — every cell, level, theme, and size \
         survives the round-trip (C2/C4)",
    );
}

/// GTW-432 C3 — a saved prefab never contains an illegal cell: a map with a SLAB on a ladder
/// (the GTW-430 illegal placement) is REJECTED by [`editor_map_to_prefab`], nothing serialized.
///
/// Pin-discriminating: without the C3 illegal-cell guard the projection would succeed and the
/// illegal slab would land in the saved prefab — this assertion would flip.
#[test]
fn illegal_cell_rejects_the_save() {
    let reg = registry();
    let session = session();
    let mut illegal = EditorMap::new();
    let size = size();

    // Construct the GTW-430 illegal state directly via the level-aware writes: a LADDER at
    // L0 and a SLAB directly ABOVE it at L1. The slab seals the ladder's destination, which
    // `evaluate_placement` flags `SlabSealsLadder` — the canvas commit would reject the slab,
    // but the model can hold it, so the save guard must catch it (C3).
    assert!(illegal.paint_at(ground(1, 1), key("steel_ladder"), size));
    assert!(illegal.paint_at(
        CellLevel::new(Cell::new(1, 1), Level::new(1)),
        key("deck_slab"),
        size,
    ));

    let built = editor_map_to_prefab(&illegal, &reg, &session);
    assert!(
        built.is_err(),
        "a map with a slab sealing a ladder must be REJECTED (C3) — got {built:?}",
    );
}
