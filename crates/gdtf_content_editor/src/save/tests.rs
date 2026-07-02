//! In-crate tests for the save path (GTW-432; swept onto the v2 UUID schema in GTW-495) —
//! sanitize, path resolution, the v2 round-trip (C2), and the illegal-cell guard (C3).

use gdtf_battle_sim::{
    Cell,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab2, PrefabName, PrefabSpecV2, SpawnRole,
        ThemeUuid,
    },
    metric::{CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

use super::project::{editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab};
use crate::{EditorMap, session::MapEditorSession};

/// The theme key the test registry is keyed by.
fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// A terrain UUID from a small constant.
const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

const FLOOR: TerrainUuid = tu(0x0184_0a91_0004);
const WALL: TerrainUuid = tu(0x0184_0a91_0002);
const SLAB: TerrainUuid = tu(0x0184_0a91_0005);
const LADDER: TerrainUuid = tu(0x0184_0a91_00aa);

/// A `4 × 4 × 2` drawable volume — wide enough for a boundary + a multi-level ladder, with a
/// `1 × 1 × 1` fallback (the constructor is fallible; the fallback keeps the test panic-free).
fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(2))
        .unwrap_or_else(|_| GridSize::default())
}

/// A ground `(cell, L0)` slot.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A COVER terrain def (a tile the vertical rules never constrain) at `graphic` "cover".
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
        on_death: None,
    }
}

/// A WALL terrain def.
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
        on_death: None,
    }
}

/// A SLAB terrain def.
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
        on_death: None,
    }
}

/// A registry with a default floor (`FLOOR`, cover-kind), a wall (`WALL`), a slab (`SLAB`), and a
/// ladder-named def (`LADDER`, cover-kind, recognised by display name).
fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (FLOOR, cover_def(FLOOR, "Deck Floor")),
        (WALL, wall_def(WALL, "Bulkhead Wall")),
        (SLAB, slab_def(SLAB, "Deck Slab")),
        (LADDER, cover_def(LADDER, "Steel Ladder")),
    ])
}

/// A session at the test theme/size with `FLOOR` as the resolved default floor.
fn session() -> MapEditorSession {
    MapEditorSession::new(theme(), Some(FLOOR), size())
}

/// `sanitize_name` folds a free-form name to the file-stem convention and rejects an empty name.
#[test]
fn sanitize_name_folds_to_stem() {
    assert_eq!(sanitize_name("  Entry Room  "), "entry_room");
    assert_eq!(sanitize_name("Sump-Waste 2!"), "sump_waste_2");
    assert!(
        sanitize_name("   ").is_empty(),
        "a blank name sanitizes to empty"
    );
}

/// The save path is `assets/content/maps/<theme>/<size>/<stem>.prefab_v2.ron` — the layout the GTW-489
/// v2 loader scans (the `.prefab_v2` infix is required so the loader keys it). The theme dir is
/// the slugified theme display name (NOT a closed-enum match).
#[test]
fn save_path_is_themed_sized_and_dot_prefab_v2_ron() {
    let path = prefab_save_path("Industrial Hive", size(), "entry_room");
    let tail: Vec<_> = path
        .components()
        .rev()
        .take(4)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    // Reversed: file, size dir, theme dir, maps dir.
    assert_eq!(
        tail[0], "entry_room.prefab_v2.ron",
        "the .prefab_v2.ron extension"
    );
    assert_eq!(tail[1], "4x4", "the <width>x<height> size dir");
    assert_eq!(
        tail[2], "industrial_hive",
        "the slugified theme display-name dir (not a closed-enum match)",
    );
    assert_eq!(tail[3], "maps", "under the v2 maps root");
}

/// GTW-495 C2 — the IDENTITY round-trip on the REAL code path: build an [`EditorMap`] with a
/// boundary floor, a wall, a slab, AND a MULTI-LEVEL ladder cell, project it via the real
/// [`editor_map_to_prefab`] + [`serialize_prefab`], deserialize via the SAME parser the GTW-489
/// loader uses (`ron::de::from_str::<PrefabSpecV2>`), validate via the SAME (infallible)
/// [`Prefab2::new`] the loader runs, and assert the reloaded spec EQUALS the saved one — every
/// cell, level, theme, and size survives, with NO authored openings in the schema.
///
/// Pin-discriminating: if a placement were dropped (a level, a cell, theme, or size) the reloaded
/// spec would differ and the `assert_eq!` would fail. The multi-level ladder placement pins the
/// level dimension specifically.
///
/// In-memory string round-trip — it NEVER writes into the real `assets/` tree. Uses `assert!` +
/// `let … else { return }`, NOT `panic!`/`unwrap` (denied lints even in tests).
#[test]
fn editor_map_round_trips_through_the_loader() {
    let reg = registry();
    let session = session();
    let mut map = EditorMap::new();
    let size = size();

    // A walkable boundary floor cell.
    assert!(map.paint(Cell::new(0, 1), FLOOR, size));
    // A wall on the back boundary corner.
    assert!(map.paint(Cell::new(3, 3), WALL, size));
    // A slab at an interior ground cell.
    assert!(map.paint(Cell::new(2, 2), SLAB, size));
    // A MULTI-LEVEL cell: a ladder at L1 — pins the level dimension.
    assert!(map.paint_at(CellLevel::new(Cell::new(1, 1), Level::new(1)), LADDER, size,));

    // SAVE via the real projection + serialize.
    let built = editor_map_to_prefab(&map, &reg, &session);
    assert!(
        built.is_ok(),
        "the map must project to a v2 prefab: {:?}",
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

    // RELOAD via the loader path: deserialize with the SAME parser RonAsset<PrefabSpecV2> uses …
    let reloaded = ron::de::from_str::<PrefabSpecV2>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized prefab must round-trip through the PrefabSpecV2 deserializer: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };

    // … and BUILD via the SAME (infallible) Prefab2::new the loader runs (no C6 edge-opening
    // check in the v2 schema) — so the round-trip exercises the loader's real build path.
    let validated = Prefab2::new(PrefabName::new("entry_room".to_owned()), reloaded.clone());
    assert_eq!(
        validated.spec(),
        &reloaded,
        "the built v2 prefab carries the reloaded spec unchanged (no validation drops data)",
    );

    // load(save(grid)) == grid: the reloaded spec equals the saved one (PrefabSpecV2: PartialEq).
    assert_eq!(
        reloaded, saved,
        "the reloaded prefab must equal the saved one — every cell, level, theme, and size \
         survives the v2 round-trip (C2)",
    );
    // Every painted cell appears as a placement (4 painted cells -> 4 placements), and there are
    // no authored openings to lose (the v2 schema has none).
    assert_eq!(
        saved.placements.len(),
        4,
        "the four painted cells must each be one placement (C2)",
    );
    // The editor has no spawn-role control, so a saved fragment is authored as the connective
    // `Fill` default (C2) — a STRUCTURAL pin (enum variant, not a tunable magnitude): it flips the
    // moment `SAVED_SPAWN_ROLE` is changed away from `Fill`, the behavioral component the clause
    // enumerates. The identity assert above can't catch this on its own (it holds for any role).
    assert_eq!(
        saved.role,
        SpawnRole::Fill,
        "a saved prefab is authored as the connective Fill default (C2)",
    );
    // … and that connective role survives the round-trip unchanged (it is not the serde default of
    // some OTHER value silently coerced back to Fill on reload).
    assert_eq!(
        reloaded.role,
        SpawnRole::Fill,
        "the connective Fill role survives the v2 round-trip (C2)",
    );
}

/// GTW-495 C3 — a saved prefab never contains an illegal cell: a map with a SLAB on a ladder
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

    // Construct the GTW-430 illegal state directly: a LADDER at L0 and a SLAB directly ABOVE it
    // at L1. The slab seals the ladder's destination (`SlabSealsLadder`).
    assert!(illegal.paint_at(ground(1, 1), LADDER, size));
    assert!(illegal.paint_at(CellLevel::new(Cell::new(1, 1), Level::new(1)), SLAB, size,));

    let built = editor_map_to_prefab(&illegal, &reg, &session);
    assert!(
        built.is_err(),
        "a map with a slab sealing a ladder must be REJECTED (C3) — got {built:?}",
    );
}
