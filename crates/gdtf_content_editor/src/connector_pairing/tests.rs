use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
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

use super::{
    PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
};
use crate::{
    editor_map::{EditorMap, PaintedPiece},
    placement::ProposedPlacement,
};

fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0002))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

fn stair_def(key: TerrainUuid, label: &str, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(label.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

const STAIR_NS_UP: TerrainUuid = tu(0x0d);
const STAIR_NS_DOWN: TerrainUuid = tu(0x0e);
const STAIR_EW_UP: TerrainUuid = tu(0x0f);
const STAIR_EW_DOWN: TerrainUuid = tu(0x10);

fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (
            STAIR_NS_UP,
            stair_def(STAIR_NS_UP, "Deck Stair Up (NS)", "stair_ns_up"),
        ),
        (
            STAIR_NS_DOWN,
            stair_def(STAIR_NS_DOWN, "Deck Stair Down (NS)", "stair_ns_down"),
        ),
        (
            STAIR_EW_UP,
            stair_def(STAIR_EW_UP, "Deck Stair Up (EW)", "stair_ew_up"),
        ),
        (
            STAIR_EW_DOWN,
            stair_def(STAIR_EW_DOWN, "Deck Stair Down (EW)", "stair_ew_down"),
        ),
    ])
}

fn at(cell: Cell, level: u8) -> CellLevel {
    CellLevel::new(cell, Level::new(level))
}

#[test]
fn up_connector_recognition_and_counterpart_resolution() {
    let reg = registry();
    assert!(
        is_up_connector(&reg, &STAIR_NS_UP),
        "stair_ns_up is an up connector"
    );
    assert!(
        is_up_connector(&reg, &STAIR_EW_UP),
        "stair_ew_up is an up connector"
    );
    assert!(
        !is_up_connector(&reg, &STAIR_NS_DOWN),
        "stair_ns_down is NOT an up connector",
    );

    assert_eq!(
        resolve_down_counterpart(&reg, &STAIR_NS_UP),
        Some(STAIR_NS_DOWN),
        "NS up pairs to NS down (symmetric by direction)",
    );
    assert_eq!(
        resolve_down_counterpart(&reg, &STAIR_EW_UP),
        Some(STAIR_EW_DOWN),
        "EW up pairs to EW down (symmetric by direction)",
    );
    assert_eq!(
        resolve_down_counterpart(&reg, &STAIR_NS_DOWN),
        None,
        "a down connector has no up→down counterpart (one-way pairing)",
    );
}

#[test]
fn out_of_vocabulary_up_name_does_not_pair() {
    const LADDER_UP: TerrainUuid = tu(0x20);
    const LADDER_DOWN: TerrainUuid = tu(0x21);
    let reg = TerrainDefRegistry::new([
        (
            LADDER_UP,
            stair_def(LADDER_UP, "Custom Ladder Up", "ladder_up"),
        ),
        (
            LADDER_DOWN,
            stair_def(LADDER_DOWN, "Custom Ladder Down", "ladder_down"),
        ),
    ]);
    assert!(
        !is_up_connector(&reg, &LADDER_UP),
        "an out-of-vocabulary `ladder_up` graphic is NOT an up connector (fail-closed)",
    );
    assert_eq!(
        resolve_down_counterpart(&reg, &LADDER_UP),
        None,
        "an out-of-vocabulary `*_up` name resolves no counterpart (no phantom pairing)",
    );
}

#[test]
fn placing_up_connector_auto_places_down_pair_above() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);

    let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_UP, TerrainFacing::East);
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PairPlaced {
            down: STAIR_NS_DOWN,
            at:   at(cell, 1),
        },
        "placing an up connector at N auto-places the down pair at N+1 (C2)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 0)),
        Some(PaintedPiece::new(STAIR_NS_UP, TerrainFacing::East)),
        "the up connector is placed at N (C2)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 1)),
        Some(PaintedPiece::new(STAIR_NS_DOWN, TerrainFacing::East)),
        "the paired down connector is auto-placed at N+1 wearing the source placement's \
         facing, never a reset default",
    );
    assert_eq!(map.painted_count(), 2, "both endpoints are present (C2)");
}

#[test]
fn up_connector_on_top_storey_skips_pair_fail_closed() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(2, 2);
    let placement = ProposedPlacement::new(at(cell, 2), STAIR_EW_UP, TerrainFacing::default());
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PlacedPairSkipped,
        "an up connector on the top storey skips the pair (fail-closed C2)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 2)),
        Some(PaintedPiece::new(STAIR_EW_UP, TerrainFacing::default())),
        "the up connector still landed on the top storey (C2)",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "only the up connector — no pair above the top (C2)"
    );
}

#[test]
fn non_connector_placement_places_no_pair() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(0, 0);
    let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_DOWN, TerrainFacing::default());
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());
    assert_eq!(
        outcome,
        PairingOutcome::PlacedNoPair,
        "a down connector places no pair"
    );
    assert_eq!(map.painted_count(), 1, "only the placed tile — no pair");
}
