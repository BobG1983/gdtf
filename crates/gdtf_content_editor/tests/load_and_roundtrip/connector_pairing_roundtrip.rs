//! Stair pairing: paint a staircase, auto-place its second end, round-trip both endpoints.
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabSpec, ThemeUuid,
    },
    metric::{CellLevel, Level},
    prelude::Cell,
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag, TerrainUuid, TerrainViews,
        },
        facing::TerrainFacing,
    },
};
use gdtf_content_editor::{
    EditorMap, MapEditorSession, PaintedPiece, PairingOutcome, ProposedPlacement,
    apply_placement_with_pairing, editor_map_to_prefab, serialize_prefab,
};

const fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0531_0000_0001))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

const STAIR: TerrainUuid = tu(0x0531_000d);

fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

fn stair_def(key: TerrainUuid, label: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab { footfall: None },
        tags: vec![TerrainTag::Stair],
        views: TerrainViews::new(Vec::new()),
        on_death: Vec::new(),

        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([(STAIR, stair_def(STAIR, "Deck Stair"))])
}

fn at(cell: Cell, level: u8) -> CellLevel {
    CellLevel::new(cell, Level::new(level))
}

#[test]
fn up_connector_pairs_down_above_and_round_trips_both_endpoints() {
    let reg = registry();
    let session = MapEditorSession::new(theme(), None, size());
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);

    let placement = ProposedPlacement::new(at(cell, 0), STAIR, TerrainFacing::default());
    let outcome = apply_placement_with_pairing(&mut map, &reg, theme(), &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PairPlaced {
            paired: STAIR,
            at:     at(cell, 1),
        },
        "painting a staircase at N must auto-place its second end at N+1 (C2/C5)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 0)),
        Some(PaintedPiece::new(STAIR, TerrainFacing::default())),
        "the painted staircase is at N",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 1)),
        Some(PaintedPiece::new(STAIR, TerrainFacing::default())),
        "the second end is auto-placed at N+1 (the auto-placement FIRED)",
    );
    assert_eq!(
        map.painted_count(),
        2,
        "both endpoints present in the editor prefab (C5)"
    );

    let built = editor_map_to_prefab(&map, &reg, &session);
    assert!(
        built.is_ok(),
        "the paired map must project to a prefab: {:?}",
        built.as_ref().err(),
    );
    let Ok(saved) = built else { return };
    let serialized = serialize_prefab(&saved);
    assert!(
        serialized.is_ok(),
        "serializing the paired prefab must succeed: {:?}",
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
    let _validated = Prefab::new(PrefabName::new("stair_room".to_owned()), reloaded.clone());

    assert_eq!(
        reloaded, saved,
        "the reloaded prefab equals the saved one — both connector endpoints survive (C3)",
    );
    assert_eq!(
        saved.placements.len(),
        2,
        "the round-tripped prefab carries BOTH endpoints (up at N + down at N+1) (C3/C5)",
    );

    let has_lower = reloaded
        .placements
        .iter()
        .any(|p| p.piece == STAIR && p.at == at(cell, 0));
    let has_upper = reloaded
        .placements
        .iter()
        .any(|p| p.piece == STAIR && p.at == at(cell, 1));
    assert!(
        has_lower,
        "the reloaded prefab has the painted staircase at N (C3)"
    );
    assert!(
        has_upper,
        "the reloaded prefab has its second end at N+1 (C3)"
    );
}
