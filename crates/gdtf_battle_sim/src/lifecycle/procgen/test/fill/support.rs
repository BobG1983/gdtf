//! Shared fixtures for the fill-pass tests — the canonical test theme / prefab / registry /
//! tuning builders and the coverage-accounting helpers the [`content`](super::content) and
//! [`termination`](super::termination) concern files reach via `super::support::…`.
//!
//! Visibility is `pub(super)` (= `pub(in super::super::fill)`): both sibling test files consume
//! the same builders, so per module-layout rule 6 ("helpers with 2+ consuming modules live in
//! the shared support/harness module") they live here rather than being duplicated.

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

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
pub(super) fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The canonical test theme key — a fixed `from_u128` [`ThemeUuid`] every test prefab
/// authors so the fill's theme-keyed candidate lookup resolves them.
pub(super) fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2427_0000_0001))
}

/// A v2 prefab of `role` at footprint `fp` under `theme`, authoring no placements (the fill
/// pass cares only about footprint + role + theme — the geometry is the emit step's concern).
fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(theme, fp, role, Vec::new()),
    )
}

/// A registry with a player + enemy deployment prefab and the given list of `Fill`
/// prefabs (each `(stem, w, h)`) under `theme`. `None` if any fill size is invalid.
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

/// A tuning with explicit knob values and NO binding coverage cap (cap `1.0`, which never
/// fires so the fill runs to the density floor or exhaustion, the pre-GTW-767 shape most of
/// these tests want).
pub(super) fn tuning(density: f32, large_area: u32, scatter_k: u8) -> ProcgenTuning {
    tuning_capped(density, large_area, scatter_k, 1.0)
}

/// A tuning with explicit knob values AND an explicit max coverage cap — the GTW-767 knob the
/// cap-termination test drives directly.
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

/// The placed-cell coverage the fill's own `coverage_fraction` measures: player + enemy +
/// every fill prefab (NOT dead space) — the numerator the density floor and the GTW-767
/// coverage cap both compare against the board's cell count.
pub(super) fn placed_cells(filled: &FilledPlacement) -> i64 {
    let mut sum = *filled.placement().player().region().cell_count()
        + *filled.placement().enemy().region().cell_count();
    for p in filled.fill() {
        sum += *p.region().cell_count();
    }
    sum
}

/// The total cells the placement + fill + dead-space regions cover (the placed cells plus
/// every dead-space region) — used to assert the playable area is never shrunk (C3: board
/// fully accounted for, no lost cells).
pub(super) fn covered_plus_dead(filled: &FilledPlacement) -> i64 {
    let mut sum = placed_cells(filled);
    for d in filled.dead_space() {
        sum += *d.cell_count();
    }
    sum
}
