use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, TerrainPlacementEntry, ThemeDisplayName, ThemeUuid, UuidThemeDef,
        UuidThemeRegistry,
    },
    metric::{Cell, CellLevel, Level},
    procgen::{
        DeadRectScatterCount, FilledPlacement, LargePrefabAreaThreshold, MaxCoverageCap,
        MinDensityFloor, MinPlayerSide, ProcgenTuning, SplitMode, assemble_placement_with,
        fill_placement_with,
    },
    rng::{BattleSeed, ProcgenRng},
    situation::BattleMap,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        facing::TerrainFacing,
    },
};

pub(in crate::lifecycle::procgen::test) fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(in crate::lifecycle::procgen::test) fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

pub(in crate::lifecycle::procgen::test) fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2431_0000_0001))
}

pub(in crate::lifecycle::procgen::test) fn wall_piece() -> TerrainUuid {
    crate::test_support::test_pieces::WALL
}

pub(in crate::lifecycle::procgen::test) fn floor_piece() -> TerrainUuid {
    crate::test_support::test_pieces::FLOOR
}

pub(in crate::lifecycle::procgen::test) fn prefab(
    theme: ThemeUuid,
    fp: GridSize,
    role: SpawnRole,
    stem: &str,
) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(
            theme,
            fp,
            role,
            vec![TerrainPlacementEntry::new(
                wall_piece(),
                at(1, 1),
                TerrainFacing::default(),
            )],
        ),
    )
}

pub(in crate::lifecycle::procgen::test) fn registry_with_fill(
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

pub(in crate::lifecycle::procgen::test) fn theme_registry(theme: ThemeUuid) -> UuidThemeRegistry {
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

pub(in crate::lifecycle::procgen::test) fn terrain_defs() -> TerrainDefRegistry {
    crate::test_support::test_terrain_registry()
}

pub(in crate::lifecycle::procgen::test) fn tuning(
    density: f32,
    large_area: u32,
    scatter_k: u8,
) -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(density),
        max_coverage_cap:            MaxCoverageCap::new(1.0),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(large_area),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(scatter_k),
    }
}

pub(in crate::lifecycle::procgen::test) fn terrain_eq(a: &BattleMap, b: &BattleMap) -> bool {
    a.theme == b.theme
        && a.grid_size == b.grid_size
        && a.default_floor == b.default_floor
        && a.walls == b.walls
        && a.scatter == b.scatter
        && a.slabs == b.slabs
        && a.floors == b.floors
        && a.vertical_links == b.vertical_links
}

pub(in crate::lifecycle::procgen::test) fn run_pipeline(
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
