//! Integration test for GTW-531 — the prefab-editor **up→down connector auto-pairing**, driven
//! end to end on the REAL public path (no reimplementation):
//!
//! 1. Place an UP connector at `(x, y, N)` via the shipped
//!    [`apply_placement_with_pairing`](gdtf_content_editor::apply_placement_with_pairing) — the
//!    SAME fn the prefab viewport click-commit runs.
//! 2. Assert the paired DOWN connector auto-appears at `(x, y, N+1)` in the [`EditorMap`].
//! 3. Project + serialize the map through the REAL save path
//!    ([`editor_map_to_prefab`](gdtf_content_editor::editor_map_to_prefab) +
//!    [`serialize_prefab`](gdtf_content_editor::serialize_prefab)) and reload it through the SAME
//!    `PrefabSpec` deserializer the GTW-489 loader uses — asserting BOTH endpoints survive (C3 /
//!    C5).
//!
//! A green build alone does NOT prove the pairing — this asserts the auto-placement FIRES and both
//! endpoints round-trip. `assert!` + `let … else` keep it panic-free per the workspace lints.

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
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};
use gdtf_content_editor::{
    EditorMap, MapEditorSession, PairingOutcome, ProposedPlacement, apply_placement_with_pairing,
    editor_map_to_prefab, serialize_prefab,
};

/// The theme key the test registry is associated with.
const fn theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0531_0000_0001))
}

/// A terrain UUID from a small constant.
const fn tu(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
}

const STAIR_NS_UP: TerrainUuid = tu(0x0531_000d);
const STAIR_NS_DOWN: TerrainUuid = tu(0x0531_000e);

/// A `4 × 4 × 3` volume — room for the N / N+1 pairing, with a `1 × 1 × 1` fallback (the
/// constructor is fallible; the fallback keeps the test panic-free).
fn size() -> GridSize {
    GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
        .unwrap_or_else(|_| GridSize::default())
}

/// A stair-like def (`sim_kind` Slab per GTW-470) whose presenter graphic name is `graphic` — the
/// up/down distinction the pairing reads. Magnitudes are throwaway data.
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
    }
}

/// A registry with the NS ascending/descending stair pair (mirrors the shipped `industrial_hive`
/// `stair_ns_up` / `stair_ns_down` graphic-name convention).
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
    ])
}

fn at(cell: Cell, level: u8) -> CellLevel {
    CellLevel::new(cell, Level::new(level))
}

/// C5 (the headline, real path): placing an UP connector at `(x, y, N)` through the shipped
/// [`apply_placement_with_pairing`] auto-places the paired DOWN connector at `(x, y, N+1)`, and the
/// `.ron` round-trip through the real save path preserves BOTH endpoints (C3).
///
/// Pin-discriminating: without the pairing the map (and the serialized prefab) would hold ONLY the
/// up connector at N — the N+1 assertions AND the two-placement round-trip assert would flip.
#[test]
fn up_connector_pairs_down_above_and_round_trips_both_endpoints() {
    let reg = registry();
    let session = MapEditorSession::new(theme(), None, size());
    let mut map = EditorMap::new();
    let cell = Cell::new(1, 1);

    // ── Place the UP connector at N via the REAL viewport-commit fn. ──────────────────────────────
    let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_UP);
    let outcome = apply_placement_with_pairing(&mut map, &reg, theme(), &placement, size());

    assert_eq!(
        outcome,
        PairingOutcome::PairPlaced {
            down: STAIR_NS_DOWN,
            at:   at(cell, 1),
        },
        "placing an up connector at N must auto-place the down pair at N+1 (C2/C5)",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 0)),
        Some(STAIR_NS_UP),
        "the up connector is placed at N",
    );
    assert_eq!(
        map.tile_at_level(at(cell, 1)),
        Some(STAIR_NS_DOWN),
        "the paired down connector is auto-placed at N+1 (the auto-placement FIRED)",
    );
    assert_eq!(
        map.painted_count(),
        2,
        "both endpoints present in the editor prefab (C5)"
    );

    // ── SAVE via the real projection + serialize. ─────────────────────────────────────────────────
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

    // ── RELOAD via the SAME parser + (infallible) builder the GTW-489 loader uses. ───────────────
    let reloaded = ron::de::from_str::<PrefabSpec>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized prefab must round-trip through the PrefabSpec deserializer: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };
    let _validated = Prefab::new(PrefabName::new("stair_room".to_owned()), reloaded.clone());

    // load(save(grid)) == grid: both endpoints survive the .ron round-trip (C3).
    assert_eq!(
        reloaded, saved,
        "the reloaded prefab equals the saved one — both connector endpoints survive (C3)",
    );
    assert_eq!(
        saved.placements.len(),
        2,
        "the round-tripped prefab carries BOTH endpoints (up at N + down at N+1) (C3/C5)",
    );

    // Explicitly confirm each endpoint is present in the reloaded placements (not just the count).
    let has_up = reloaded
        .placements
        .iter()
        .any(|p| p.piece == STAIR_NS_UP && p.at == at(cell, 0));
    let has_down = reloaded
        .placements
        .iter()
        .any(|p| p.piece == STAIR_NS_DOWN && p.at == at(cell, 1));
    assert!(has_up, "the reloaded prefab has the UP connector at N (C3)");
    assert!(
        has_down,
        "the reloaded prefab has the paired DOWN connector at N+1 (C3)"
    );
}
