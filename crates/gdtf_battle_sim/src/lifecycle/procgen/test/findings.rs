//! GTW-582 C3(d) pins for the emit step's ENGAGEMENT-TIME findings: the two degraded
//! resolutions [`emit_level`](crate::procgen::emit_level) can take while pouring a level
//! must ride back on [`EmittedLevel::findings`](crate::procgen::EmittedLevel) — never
//! silently. Both tests drive the REAL [`generate_level`] pipeline (no shadow walker):
//! reverting `EmittedLevel::findings` to always-empty fails both.

use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, LargePrefabAreaThreshold, MinDensityFloor, ProcgenFinding,
        ProcgenTuning, generate_level,
    },
    rng::{BattleSeed, ProcgenRng},
    terrain::def::TerrainUuid,
};

/// A `(cell, level)` on level 0.
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The fixture theme key (distinct from the sibling emit tests' key).
fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_0582_0000_0001))
}

/// A terrain UUID deliberately ABSENT from the test terrain-def registry — the
/// dangling placed piece the fail-open pour must report.
const GHOST_PIECE: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_0582_0000_00FF));

/// The fixture tuning (the values the sibling emit tests drive).
fn tuning() -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(0.8),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(49),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(2),
    }
}

/// A player + enemy prefab registry whose prefabs author `placements` — so the emit pours
/// real placed pieces through its real classification path.
fn registry_placing(
    theme: ThemeUuid,
    fp: GridSize,
    placements: &[(TerrainUuid, i32)],
) -> PrefabRegistry {
    let placed = |role: SpawnRole, stem: &str| {
        Prefab::new(
            PrefabName::new(stem.to_owned()),
            PrefabSpec::new(
                theme,
                fp,
                role,
                placements
                    .iter()
                    .map(|(piece, x)| TerrainPlacementEntry::new(*piece, at(*x, 1)))
                    .collect(),
            ),
        )
    };
    let mut prefabs = PrefabRegistry::default();
    prefabs.insert(placed(SpawnRole::Player, "player_pad"));
    prefabs.insert(placed(SpawnRole::Enemy, "enemy_pad"));
    prefabs
}

/// A theme registry naming `theme` with the canonical test FLOOR piece as its default floor.
fn theme_registry(theme: ThemeUuid) -> UuidThemeRegistry {
    UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Findings Test Theme".to_owned()),
            default_floor: crate::test_support::test_pieces::FLOOR,
            terrain:       vec![
                crate::test_support::test_pieces::WALL,
                crate::test_support::test_pieces::FLOOR,
            ],
        },
    )])
}

/// GTW-582 C3(d) — a theme ABSENT from the [`UuidThemeRegistry`] pours the NIL-sentinel
/// `default_floor` (the last-resort degraded pour) AND rides back as a
/// [`ProcgenFinding::MissingThemeDefaultFloor`] on [`EmittedLevel::findings`](crate::procgen::EmittedLevel)
/// — never silently. Control: the SAME generation against a registry that DOES resolve the
/// theme emits ZERO findings (the clean pour stays clean).
///
/// Pin-discriminating: reverting `EmittedLevel::findings` to always-empty fails the positive
/// containment assert; pushing findings unconditionally fails the clean-pour control.
#[test]
fn missing_theme_default_floor_pours_nil_and_is_reported() {
    let theme = theme();
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    let prefabs = registry_placing(theme, fp, &[(crate::test_support::test_pieces::WALL, 1)]);
    let terrain_defs = crate::test_support::test_terrain_registry();
    let knobs = tuning();
    let seed = BattleSeed::new(0x0582_C3D1);

    // Degraded pour: the theme resolves NO registry entry (empty theme registry).
    let mut rng = ProcgenRng::from_root(seed);
    let Ok(degraded) = generate_level(
        &prefabs,
        &UuidThemeRegistry::default(),
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    ) else {
        return;
    };
    assert!(
        *degraded.situation.default_floor.is_nil(),
        "a theme absent from the registry must pour the NIL-sentinel default_floor (the \
         last-resort degraded pour)",
    );
    assert!(
        degraded
            .findings
            .contains(&ProcgenFinding::MissingThemeDefaultFloor { theme }),
        "the nil-sentinel pour must ride back as a MissingThemeDefaultFloor finding naming \
         the unresolved theme (C3(d) — never silent); got {:?}",
        degraded.findings,
    );

    // Clean-pour control: the same generation with the theme RESOLVED emits zero findings.
    let mut rng = ProcgenRng::from_root(seed);
    let Ok(clean) = generate_level(
        &prefabs,
        &theme_registry(theme),
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    ) else {
        return;
    };
    assert!(
        clean.findings.is_empty(),
        "a fully-resolved pour must emit ZERO findings (the clean path stays clean); got {:?}",
        clean.findings,
    );
}

/// GTW-582 C3(d) — a placed piece whose UUID resolves NO `TerrainDef` is poured FAIL-OPEN
/// into the `walls` list (it must surface at setup, never vanish) AND is reported as a
/// [`ProcgenFinding::UnresolvedTerrainPiece`] exactly ONCE per unique UUID — the pour
/// deduplicates however many cells place that UUID (here 2 cells x player + enemy = 4 pours).
///
/// Pin-discriminating: reverting `EmittedLevel::findings` to always-empty fails the
/// containment assert; dropping the dedup (one finding per POUR) fails the exactly-once
/// assert; silently DROPPING the unresolved piece (the pre-GTW-582 filter behavior this
/// clause folds in) fails the fail-open walls assert.
#[test]
fn unresolved_terrain_piece_pours_fail_open_and_is_reported_once() {
    let theme = theme();
    let (Some(board), Some(fp)) = (size(40, 40), size(12, 12)) else {
        return;
    };
    // Each prefab places the SAME unknown UUID at TWO cells plus one known wall — so the
    // dedup is exercised across cells AND across prefabs, with a resolved-piece control.
    let prefabs = registry_placing(
        theme,
        fp,
        &[
            (GHOST_PIECE, 1),
            (GHOST_PIECE, 2),
            (crate::test_support::test_pieces::WALL, 3),
        ],
    );
    let themes = theme_registry(theme);
    let terrain_defs = crate::test_support::test_terrain_registry();
    let knobs = tuning();

    let mut rng = ProcgenRng::from_root(BattleSeed::new(0x0582_C3D2));
    let Ok(emitted) = generate_level(
        &prefabs,
        &themes,
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    ) else {
        return;
    };

    // Fail-open: every unresolved placement still poured into walls (2 cells x 2 prefabs).
    let ghost_walls = emitted
        .situation
        .walls
        .iter()
        .filter(|w| w.piece == GHOST_PIECE)
        .count();
    assert_eq!(
        ghost_walls, 4,
        "every unresolved placement must pour FAIL-OPEN into the walls list (2 cells x \
         player + enemy prefabs), never be silently dropped",
    );

    // Reported exactly ONCE per unique UUID (deduplicated across the 4 pours) — and the
    // resolved WALL control piece produced no finding.
    let ghost_findings = emitted
        .findings
        .iter()
        .filter(|f| matches!(f, ProcgenFinding::UnresolvedTerrainPiece { piece } if *piece == GHOST_PIECE))
        .count();
    assert_eq!(
        ghost_findings, 1,
        "an unresolved placed UUID must be reported exactly ONCE however many cells place \
         it (deduplicated, C3(d)); got {:?}",
        emitted.findings,
    );
    assert_eq!(
        emitted.findings.len(),
        1,
        "the resolved pieces (the known WALL, the theme's floor) must produce NO finding; \
         got {:?}",
        emitted.findings,
    );
}
