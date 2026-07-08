//! GTW-662 C2/A2: the PREFAB save's REAL fs round-trip — author an [`EditorMap`] through the
//! shipped placement predicate, save it through the REAL root-parameterized write
//! (`write_prefab_in`) into a `TempDir` assets root (the GTW-555 pattern — the shipped
//! `assets/` tree is NEVER written), then boot the REAL game Load chain rooted at that
//! directory and assert the actual GTW-489 `resolve_prefabs` folder walk loads the saved
//! fragment back structurally identical into the [`PrefabRegistry`].
//!
//! This is the FIRST test to exercise `write_prefab`'s filesystem half (the GTW-653 census
//! finding): before GTW-662 the writer had no `TempDir` seam, so its fs half was untestable
//! without polluting the version-controlled `assets/` tree. Prefabs are a declared GTW-570
//! seam EXCLUSION — their loader is the game's bespoke Load branch (`gdtf_app`), not a
//! `ContentFamily` — so the reload half boots the game's Load orchestration via
//! [`GdtfLoadTestAppBuilder`] (the `load_prefab.rs` harness) instead of the editor app the
//! sibling `*_mode.rs` round-trips boot. Only `content/maps/` is materialized, so every
//! other family fails closed (the no-strand guarantee). `assert!` + `let … else` keep the
//! test panic-free per the workspace lints.

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
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};
use gdtf_content_editor::{
    EditorMap, MapEditorSession, ProposedPlacement, apply_placement, editor_map_to_prefab,
    write_prefab_in,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the real-asset waits gated on an async load resolving; the
/// test polls the [`PrefabRegistry`]-present SIGNAL, never a fixed frame count (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// A stable [`ThemeUuid`] fixture the authored prefab is bucketed under — mechanism, not
/// balance.
const THEME: ThemeUuid = ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0011));

/// A stable [`TerrainUuid`] for the one painted slab piece.
const SLAB: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0662_0000_0012));

/// The 3x3x1 footprint the test authors, or `None` (assert-fail) on a bad span.
fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

/// A slab-kind def for the painted piece (the `connector_pairing_roundtrip.rs` def shape) —
/// legal on the ground level, so the save's shared `evaluate_placement` re-check passes.
/// Magnitudes are throwaway mechanism data.
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
        tags:           Vec::new(),
        on_death:       None,
    }
}

/// GTW-662 — author (the REAL `apply_placement` commit) → save (the REAL `write_prefab_in`
/// into a `TempDir` assets root) → load through the REAL game Load chain (`resolve_prefabs`)
/// → the [`PrefabRegistry`] buckets the fragment under its authored `(theme, size, role)`
/// key with the SAME spec and the sanitized file-stem name.
#[test]
fn saved_prefab_round_trips_through_the_real_game_prefab_loader() {
    let Some(size) = small_size() else {
        unreachable!("the 3x3x1 span is a valid GridSize")
    };
    let registry = TerrainDefRegistry::new([(SLAB, slab_def())]);
    let session = MapEditorSession::new(THEME, None, size);

    // AUTHOR through the REAL viewport-commit predicate (the same fn the paint click runs).
    let mut map = EditorMap::new();
    let slot = CellLevel::new(Cell::new(1, 1), Level::new(0));
    let placement = ProposedPlacement::new(slot, SLAB);
    assert!(
        apply_placement(&mut map, &registry, THEME, &placement, size),
        "painting a slab on the ground level of an empty 3x3x1 map must be legal",
    );

    // The expected spec is the SAME projection write_prefab_in serializes.
    let expected = editor_map_to_prefab(&map, &registry, &session);
    assert!(
        expected.is_ok(),
        "the painted map must project to a PrefabSpec: {:?}",
        expected.as_ref().err(),
    );
    let Ok(expected) = expected else { return };

    // SAVE through the real root-parameterized write; the raw name sanitizes to the stem
    // (`tempdir entry` → `tempdir_entry`) the loader keys the prefab by.
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

    // RELOAD through the REAL game Load chain rooted at the TempDir. No `starting_in`
    // override: the app boots in `AppState::Init`, whose scene auto-advances to `Load`
    // (the production flow), and `Load`'s kick-off walks `content/maps/` for real.
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(dir.path().to_path_buf()).build();

    // Signal-poll until the REAL resolve_prefabs branch publishes the registry.
    // DELIBERATELY not seeded — existence proves the resolve branch built it from the
    // TempDir folder.
    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    let loaded = app.world().get_resource::<PrefabRegistry>();
    assert!(loaded.is_some(), "the PrefabRegistry must resolve");
    let Some(loaded) = loaded else { return };
    // The saved fragment buckets under its authored (theme, size, Fill-default role) key…
    let bucket = loaded.prefabs_for(&PrefabKey::new(THEME, size, SpawnRole::Fill));
    assert_eq!(
        bucket.len(),
        1,
        "exactly the one saved fragment must bucket under (authored theme, 3x3x1, Fill)",
    );
    let Some(prefab) = bucket.first() else { return };
    // …keyed by the sanitized file stem with the `.prefab` infix stripped…
    assert_eq!(
        prefab.name(),
        &PrefabName::new("tempdir_entry".to_owned()),
        "the loader must key the prefab by the sanitized save stem",
    );
    // …and carrying the SAME spec the editor projected (theme / size / role / placements).
    assert_eq!(
        prefab.spec(),
        &expected,
        "the reloaded prefab spec must equal the saved projection — the GTW-489 round-trip \
         through the REAL game loader",
    );
}
