//! Shared emit-pipeline fixtures — the canonical test theme / prefab / registry /
//! tuning builders and the staged-pipeline driver each concern file reaches via
//! `use super::support::*;`.

use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, FilledPlacement, LargePrefabAreaThreshold, MinDensityFloor,
        MinPlayerSide, ProcgenTuning, SplitMode, assemble_placement_with, fill_placement_with,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

/// A `(cell, level)` on level 0 (a tiny helper).
pub(super) fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
pub(super) fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The canonical test theme key — a fixed `from_u128` [`ThemeUuid`] every prefab + the
/// `UuidThemeRegistry` author, so the generated level's theme resolves to its default floor.
pub(super) fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2431_0000_0001))
}

/// The test WALL terrain def UUID a prefab places (the canonical sim test-support `WALL`
/// piece, a `Wall` sim-kind — so the emit classifies it into the `walls` list).
pub(super) fn wall_piece() -> TerrainUuid {
    crate::test_support::test_pieces::WALL
}

/// The test FLOOR terrain def UUID the theme nominates as its default floor (the
/// canonical sim test-support `FLOOR` piece).
pub(super) fn floor_piece() -> TerrainUuid {
    crate::test_support::test_pieces::FLOOR
}

/// A v2 prefab of `role` at footprint `fp` that AUTHORS a wall placement at the
/// footprint-local cell `(1, 1)` — so the emit has real terrain to translate (and so
/// different anchors, i.e. different placed origins, produce different translated cells: the
/// determinism pin is then discriminating).
pub(super) fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(
            theme,
            fp,
            role,
            vec![TerrainPlacementEntry::new(wall_piece(), at(1, 1))],
        ),
    )
}

/// A registry with a player + enemy deployment prefab and the given `Fill` prefabs (each
/// `(stem, w, h)`), every prefab authoring a wall so the emit produces real terrain.
pub(super) fn registry_with_fill(
    theme: ThemeUuid,
    player_fp: GridSize,
    enemy_fp: GridSize,
    fills: &[(&str, u8, u8)],
) -> Option<PrefabRegistry> {
    let mut r = PrefabRegistry::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad"));
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad"));
    for (stem, w, h) in fills {
        let fp = size(*w, *h)?;
        r.insert(prefab(theme, fp, SpawnRole::Fill, stem));
    }
    Some(r)
}

/// A theme registry naming `theme()` with the test FLOOR piece as its default floor — so the
/// emitted level's `default_floor` resolves to a real (non-nil) terrain UUID (GTW-492).
pub(super) fn theme_registry(theme: ThemeUuid) -> UuidThemeRegistry {
    UuidThemeRegistry::new([(
        theme,
        UuidThemeDef {
            key:           theme,
            display_name:  ThemeDisplayName::new("Test Theme".to_owned()),
            default_floor: floor_piece(),
            terrain:       vec![wall_piece(), floor_piece()],
        },
    )])
}

/// The canonical test terrain-def registry (WALL / SLAB / COVER / FLOOR) the emit classifies
/// placed pieces against (`wall_piece()` resolves to a `Wall` sim-kind → the walls list).
pub(super) fn terrain_defs() -> TerrainDefRegistry {
    crate::test_support::test_terrain_registry()
}

/// A tuning with explicit knob values (the unit tests drive the knobs directly).
pub(super) fn tuning(density: f32, large_area: u32, scatter_k: u8) -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(density),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(large_area),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(scatter_k),
    }
}

/// Whether two emitted situations have STRUCTURALLY EQUAL terrain entries — the C2
/// equality measure (the terrain leaves all derive `Eq`; `Situation` itself does not, so we
/// compare the terrain-entry fields directly).
pub(super) fn terrain_eq(a: &Situation, b: &Situation) -> bool {
    a.theme == b.theme
        && a.grid_size == b.grid_size
        && a.default_floor == b.default_floor
        && a.walls == b.walls
        && a.scatter == b.scatter
        && a.slabs == b.slabs
        && a.floors == b.floors
        && a.vertical_links == b.vertical_links
}

/// Drive the REAL staged pipeline (`assemble_placement_with` then `fill_placement_with` — the
/// exact functions [`generate_level`] composes) under a fixed seed and return the
/// [`FilledPlacement`]. Uses the RULED defaults ([`SplitMode::default`],
/// [`MinPlayerSide::DEFAULT`]) so the placement matches `generate_level`'s. Returns `None` on
/// any packing error (the caller returns early — no panic).
pub(super) fn run_pipeline(
    prefabs: &PrefabRegistry,
    theme: ThemeUuid,
    board: GridSize,
    seed: BattleSeed,
    knobs: &ProcgenTuning,
) -> Option<FilledPlacement> {
    let mut rng = ProcgenRng::from_root(seed);
    let placement = assemble_placement_with(
        prefabs,
        theme,
        board,
        &mut rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )
    .ok()?;
    fill_placement_with(
        placement,
        prefabs,
        theme,
        board,
        knobs,
        &mut rng,
        SplitMode::default(),
    )
    .ok()
}
