use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, ThemeUuid,
    },
    procgen::{
        DeadRectScatterCount, FilledPlacement, LargePrefabAreaThreshold, MaxCoverageCap,
        MinDensityFloor, ProcgenTuning,
    },
};

pub(super) fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

pub(super) fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2427_0000_0001))
}

fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(theme, fp, role, Vec::new()),
    )
}

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

pub(super) fn tuning(density: f32, large_area: u32, scatter_k: u8) -> ProcgenTuning {
    tuning_capped(density, large_area, scatter_k, 1.0)
}

pub(super) fn tuning_capped(
    density: f32,
    large_area: u32,
    scatter_k: u8,
    cap: f32,
) -> ProcgenTuning {
    ProcgenTuning {
        min_density_floor:           MinDensityFloor::new(density),
        max_coverage_cap:            MaxCoverageCap::new(cap),
        large_prefab_area_threshold: LargePrefabAreaThreshold::new(large_area),
        dead_rect_scatter_count_k:   DeadRectScatterCount::new(scatter_k),
    }
}

pub(super) fn placed_cells(filled: &FilledPlacement) -> i64 {
    let mut sum = *filled.placement().player().region().cell_count()
        + *filled.placement().enemy().region().cell_count();
    for p in filled.fill() {
        sum += *p.region().cell_count();
    }
    sum
}

pub(super) fn covered_plus_dead(filled: &FilledPlacement) -> i64 {
    let mut sum = placed_cells(filled);
    for d in filled.dead_space() {
        sum += *d.cell_count();
    }
    sum
}
