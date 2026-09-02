//! C2/A2: the PREFAB save's REAL fs round-trip — author an [`EditorMap`] through the
//! without polluting the version-controlled `assets/` tree. Prefabs are a declared
#![cfg(debug_assertions)]

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabName, PrefabRegistry,
        SpawnRole, ThemeUuid,
    },
    metric::{Cell, CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
};
use gdtf_content_editor::{
    EditorMap, MapEditorSession, ProposedPlacement, apply_placement, editor_map_to_prefab,
    write_prefab_in,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

const THEME: ThemeUuid = ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0011));

const SLAB: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0012));

/// Painted deliberately off the [`TerrainFacing`] default, so a dropped field cannot pass.
const PAINTED_FACING: TerrainFacing = TerrainFacing::East;

fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

fn slab_def() -> TerrainDef {
    TerrainDef {
        key:            SLAB,
        display_name:   TerrainDisplayName::new("Roundtrip Slab".to_owned()),
        sim_kind:       TerrainSimKind::Slab {
            hp:               SlabHp::new(100),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),

        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

#[test]
fn saved_prefab_round_trips_through_the_real_game_prefab_loader() {
    let Some(size) = small_size() else {
        unreachable!("the 3x3x1 span is a valid GridSize")
    };
    let registry = TerrainDefRegistry::new([(SLAB, slab_def())]);
    let session = MapEditorSession::new(THEME, None, size);

    let mut map = EditorMap::new();
    let slot = CellLevel::new(Cell::new(1, 1), Level::new(0));
    let placement = ProposedPlacement::new(slot, SLAB, PAINTED_FACING);
    assert!(
        apply_placement(&mut map, &registry, THEME, &placement, size),
        "painting a slab on the ground level of an empty 3x3x1 map must be legal",
    );

    let expected = editor_map_to_prefab(&map, &registry, &session);
    assert!(
        expected.is_ok(),
        "the painted map must project to a PrefabSpec: {:?}",
        expected.as_ref().err(),
    );
    let Ok(expected) = expected else { return };

    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    let written = write_prefab_in(
        dir.path(),
        &map,
        &registry,
        &session,
        "Roundtrip Hive",
        "tempdir entry",
    );
    assert!(
        written.is_ok(),
        "the real prefab write must succeed: {:?}",
        written.as_ref().err(),
    );
    if let Ok(path) = &written {
        assert!(
            path.ends_with("content/maps/roundtrip_hive/3x3/tempdir_entry.prefab.ron"),
            "the saved prefab must land on the loader's nested <theme>/<size> path; got {}",
            path.display(),
        );
    }

    let mut app = GdtfLoadTestAppBuilder::with_asset_root(dir.path().to_path_buf()).build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app);

    let loaded = app.world().get_resource::<PrefabRegistry>();
    assert!(loaded.is_some(), "the PrefabRegistry must resolve");
    let Some(loaded) = loaded else { return };
    let bucket = loaded.prefabs_for(&PrefabKey::new(THEME, size, SpawnRole::Fill));
    assert_eq!(
        bucket.len(),
        1,
        "exactly the one saved fragment must bucket under (authored theme, 3x3x1, Fill)",
    );
    let Some(prefab) = bucket.first() else { return };
    assert_eq!(
        prefab.name(),
        &PrefabName::new("tempdir_entry".to_owned()),
        "the loader must key the prefab by the sanitized save stem",
    );
    assert_eq!(
        prefab.spec(),
        &expected,
        "the reloaded prefab spec must equal the saved projection — the round-trip \
         through the REAL game loader",
    );

    let entry = prefab.spec().placements.first();
    assert_eq!(
        entry.map(|entry| entry.facing),
        Some(PAINTED_FACING),
        "the reloaded entry must carry the facing that was PAINTED, asserted against the \
         literal rather than against `expected` (which is itself the save projection, so a \
         dropped facing would show up on both sides); got {entry:?}",
    );
}
