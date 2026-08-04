//! — the authored per-def blocking OVERRIDES (`blocks_pathing` / `blocks_los`) and
use bevy::math::Vec3;

use super::super::{
    BlocksPathingOverride, LosBlocking, TerrainDef, TerrainDisplayName, TerrainPresenterKind,
    TerrainSimKind, TerrainTag, TerrainUuid, derives_path_blocking, derives_vision_occlusion,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, CoverLedger, HeightBand},
    injuries::MovementCostFactor,
    march::{MarchDir, MarchGrids, MarchKind, MarchResult, march_vector},
    metric::{Cell, CellLevel, Level, SimPos},
    occupancy::{OccupancyGrid, pathable_neighbors},
    slab::SlabHp,
    surface::SurfaceGrid,
    terrain::{floor::FloorCostGrid, piece::TerrainGraphicKey},
    tuning::{CombatTuning, MoveCosts},
};

fn key(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn slab_def(blocks_pathing: Option<bool>, blocks_los: Option<LosBlocking>) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Test Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(80),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("floor".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,
        blocks_pathing: blocks_pathing.map(BlocksPathingOverride::new),
        blocks_los,
    }
}

fn wall_def(
    band: HeightBand,
    tags: Vec<TerrainTag>,
    blocks_pathing: Option<bool>,
    blocks_los: Option<LosBlocking>,
) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Test Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
            height_band:      band,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags,
        on_death: None,
        blocks_pathing: blocks_pathing.map(BlocksPathingOverride::new),
        blocks_los,
    }
}

fn apply_derived(grid: &mut OccupancyGrid, def: &TerrainDef, at: CellLevel) {
    if *derives_path_blocking(def) {
        grid.set_path_blocking(at);
    }
    if let Some(band) = derives_vision_occlusion(def) {
        grid.set_vision_blocking(at, band);
    }
}

fn low_above(tuning: &CombatTuning) -> f32 {
    *tuning.projectile_band_edges.low_mid * 0.5
}

fn high_above(tuning: &CombatTuning) -> f32 {
    f32::midpoint(*tuning.projectile_band_edges.mid_high, 1.0)
}

fn march_row(grid: &OccupancyGrid, tuning: &CombatTuning, above: f32) -> MarchResult {
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let muzzle = SimPos::new(2.5, 10.5, above);
    march_vector(
        muzzle,
        MarchDir::new(Vec3::new(1.0, 0.0, 0.0)),
        MarchGrids {
            occupancy: grid,
            surface:   &surface,
            cover:     &cover,
        },
        tuning,
        key(2, 10),
        |_| false,
    )
}

fn walkable(grid: &OccupancyGrid, origin: CellLevel, target: CellLevel) -> bool {
    let floor = FloorCostGrid::new(MoveCosts::default().open, []);
    pathable_neighbors(origin, grid, &floor, MovementCostFactor::IDENTITY).any(|(c, _)| c == target)
}

#[test]
fn pathing_only_blocker_is_los_transparent() {
    let def = slab_def(Some(true), None);
    assert!(
        *derives_path_blocking(&def),
        "blocks_pathing: Some(true) forces the Slab to block the path",
    );
    assert_eq!(
        derives_vision_occlusion(&def),
        None,
        "no LoS override + a Slab kind default = LoS-transparent",
    );

    let occluder = key(6, 10);
    let mut grid = OccupancyGrid::new();
    apply_derived(&mut grid, &def, occluder);

    assert!(
        !walkable(&grid, key(5, 10), occluder),
        "AC2(a): the REAL pathfinder excludes the path-blocked cell",
    );

    let tuning = CombatTuning::default();
    let result = march_row(&grid, &tuning, low_above(&tuning));
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "AC2(a): the REAL march flies THROUGH the pathing-only blocker (LoS-transparent)",
    );
}

#[test]
fn los_only_blocker_is_walkable() {
    let def = slab_def(Some(false), Some(LosBlocking::Full));
    assert!(
        !*derives_path_blocking(&def),
        "blocks_pathing: Some(false) forces the Slab walkable",
    );
    assert_eq!(
        derives_vision_occlusion(&def),
        Some(HeightBand::High),
        "blocks_los: Some(Full) occludes the whole storey (resolves to High)",
    );

    let occluder = key(6, 10);
    let mut grid = OccupancyGrid::new();
    apply_derived(&mut grid, &def, occluder);

    assert!(
        walkable(&grid, key(5, 10), occluder),
        "AC2(b): the REAL pathfinder walks onto the LoS-only (walkable) blocker",
    );

    let tuning = CombatTuning::default();
    let result = march_row(&grid, &tuning, high_above(&tuning));
    assert_eq!(
        result.kind,
        MarchKind::Slab,
        "AC2(b): a HIGH round is occluded by the Full LoS blocker",
    );
    assert_eq!(result.at, occluder, "occluded exactly at the blocker cell");
}

#[test]
fn band_limited_los_on_wall_lets_high_round_over() {
    let tuning = CombatTuning::default();
    let occluder = key(6, 10);

    let limited = wall_def(
        HeightBand::Low,
        Vec::new(),
        None,
        Some(LosBlocking::UpToHeightBand),
    );
    assert_eq!(
        derives_vision_occlusion(&limited),
        Some(HeightBand::Low),
        "UpToHeightBand on a Low-band wall occludes only at Low",
    );
    let mut limited_grid = OccupancyGrid::new();
    apply_derived(&mut limited_grid, &limited, occluder);
    assert_eq!(
        march_row(&limited_grid, &tuning, low_above(&tuning)).kind,
        MarchKind::Slab,
        "AC2(c): a LOW round is occluded by the Low-band LoS blocker",
    );
    assert_eq!(
        march_row(&limited_grid, &tuning, high_above(&tuning)).kind,
        MarchKind::Miss,
        "AC2(c): a HIGH round sails OVER the band-limited LoS blocker",
    );

    let default_wall = wall_def(HeightBand::Low, Vec::new(), None, None);
    assert_eq!(
        derives_vision_occlusion(&default_wall),
        Some(HeightBand::High),
        "a Wall's kind default occludes the whole storey (Full → High)",
    );
    let mut default_grid = OccupancyGrid::new();
    apply_derived(&mut default_grid, &default_wall, occluder);
    assert_eq!(
        march_row(&default_grid, &tuning, high_above(&tuning)).kind,
        MarchKind::Slab,
        "without the override, the HIGH round IS occluded — the override changed the march",
    );
}

#[test]
fn override_beats_tags() {
    let path_off = slab_def(Some(false), None);
    let mut tagged_path = path_off;
    tagged_path.tags = vec![TerrainTag::BlocksPathfinding];
    assert!(
        !*derives_path_blocking(&tagged_path),
        "blocks_pathing: Some(false) beats a BlocksPathfinding tag",
    );

    let los_off = wall_def(
        HeightBand::High,
        vec![TerrainTag::BlocksVision],
        None,
        Some(LosBlocking::None),
    );
    assert_eq!(
        derives_vision_occlusion(&los_off),
        None,
        "blocks_los: Some(None) beats a BlocksVision tag (LoS-transparent)",
    );
}
