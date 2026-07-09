//! In-crate tests for the connector auto-pairing (GTW-531; typed GTW-566 C6): the recognition +
//! counterpart resolution, the fail-closed out-of-vocabulary case, and the C2 pair placement.

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
        piece::TerrainGraphicKey,
    },
};

use super::{
    PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
};
use crate::{editor_map::EditorMap, placement::ProposedPlacement};

fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0002))
}

const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

/// A `4 × 4 × 3` volume — enough storeys for the N / N+1 pairing, with a fallback.
fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

/// A stair-like def (`sim_kind` Slab, per GTW-470) whose graphic name is `graphic`. Magnitudes
/// are throwaway data (not pinned).
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

/// A registry mirroring the shipped `industrial_hive` stair set (up/down × NS/EW).
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

/// C1: an up-stair graphic classifies as an up connector; its down counterpart does NOT, and
/// the counterpart resolves symmetrically by direction. (GTW-566 C6: the assertions are
/// unchanged from the GTW-531 suffix-surgery era — same names, same outcomes — but the path
/// under test is now typed: `TileRole::from_key` → `is_up_connector` / `counterpart`.)
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

/// GTW-566 C6 (fail-closed): an OUT-OF-VOCABULARY `*_up` graphic name no longer
/// phantom-pairs — under the retired suffix surgery a `ladder_up`/`ladder_down` def pair
/// WOULD have paired; through the typed vocabulary it classifies to no role, so it is not a
/// connector and resolves no counterpart.
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

/// C2 (the headline): placing an UP connector at `(x, y, N)` auto-places its paired DOWN
/// connector at `(x, y, N+1)` through the shared placement predicate.
///
/// Pin-discriminating: without the pairing the map would hold ONLY the up connector at N and
/// nothing at N+1 — every N+1 assertion here would flip.
#[test]
fn placing_up_connector_auto_places_down_pair_above() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);

    let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_UP);
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
        Some(STAIR_NS_UP),
        "the up connector is placed at N (C2)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 1)),
        Some(STAIR_NS_DOWN),
        "the paired down connector is auto-placed at N+1 (C2)",
    );
    assert_eq!(map.painted_count(), 2, "both endpoints are present (C2)");
}

/// C2 fail-closed: placing an up connector on the TOP storey places only it — the pair is
/// skipped, never placed above the prefab's level range.
#[test]
fn up_connector_on_top_storey_skips_pair_fail_closed() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(2, 2);
    // size() has 3 levels (0..=2); N = 2 is the top storey, so N+1 = 3 is out of range.
    let placement = ProposedPlacement::new(at(cell, 2), STAIR_EW_UP);
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PlacedPairSkipped,
        "an up connector on the top storey skips the pair (fail-closed C2)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 2)),
        Some(STAIR_EW_UP),
        "the up connector still landed on the top storey (C2)",
    );
    assert_eq!(
        map.painted_count(),
        1,
        "only the up connector — no pair above the top (C2)"
    );
}

/// A non-connector placement is a plain single placement (no spurious pair).
#[test]
fn non_connector_placement_places_no_pair() {
    let reg = registry();
    let th = theme();
    let mut map = EditorMap::new();
    let cell = Cell::new(0, 0);
    // stair_ns_down is not an UP connector — placing it pairs nothing.
    let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_DOWN);
    let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());
    assert_eq!(
        outcome,
        PairingOutcome::PlacedNoPair,
        "a down connector places no pair"
    );
    assert_eq!(map.painted_count(), 1, "only the placed tile — no pair");
}
