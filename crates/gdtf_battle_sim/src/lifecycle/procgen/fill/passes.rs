use bevy::prelude::Deref;

use super::super::{
    anchor::Anchor,
    assembler::PlacedPrefab,
    geometry::{CellCount, Footprint, RegionRect},
    packer::MaxRectsPacker,
    tuning::ProcgenTuning,
};
use crate::level::{Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid};

const DEAD_RECT_MIN_SIDE: i32 = 4;

#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct CoverageFraction(f32);

impl CoverageFraction {
    const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BucketIndex(usize);

impl BucketIndex {
    pub(super) const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeadRectCount(usize);

impl DeadRectCount {
    const fn new(count: usize) -> Self {
        Self(count)
    }
}

pub(super) fn partition_fill_candidates(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    tuning: &ProcgenTuning,
) -> (Vec<Prefab>, Vec<Prefab>) {
    let mut all: Vec<Prefab> = registry
        .keys()
        .filter(|k| k.theme == theme && k.role == SpawnRole::Fill)
        .flat_map(|k: &PrefabKey| registry.prefabs_for(k).iter().cloned())
        .collect();
    all.sort_by(|a, b| {
        let area = |p: &Prefab| Footprint::of(p.spec().size).area();
        area(b)
            .cmp(&area(a))
            .then_with(|| (**a.name()).cmp(&**b.name()))
    });

    let threshold = tuning.large_prefab_area_threshold.area();
    let mut large = Vec::new();
    let mut small = Vec::new();
    for prefab in all {
        if Footprint::of(prefab.spec().size).area() >= threshold {
            large.push(prefab);
        } else {
            small.push(prefab);
        }
    }
    (large, small)
}

pub(super) fn try_any_other_once(
    bucket: &[Prefab],
    skip: BucketIndex,
    packer: &mut MaxRectsPacker,
) -> Option<PlacedPrefab> {
    for (i, prefab) in bucket.iter().enumerate() {
        if i == *skip {
            continue;
        }
        if let Some(placed) = place_in_largest_free(packer, prefab) {
            return Some(placed);
        }
    }
    None
}

pub(super) fn dead_rect_count(packer: &MaxRectsPacker) -> DeadRectCount {
    let count = packer
        .free_rects()
        .iter()
        .filter(|r| {
            r.footprint().width() >= DEAD_RECT_MIN_SIDE
                && r.footprint().height() >= DEAD_RECT_MIN_SIDE
        })
        .count();
    DeadRectCount::new(count)
}

pub(super) fn place_in_largest_free(
    packer: &mut MaxRectsPacker,
    prefab: &Prefab,
) -> Option<PlacedPrefab> {
    let footprint = Footprint::of(prefab.spec().size);
    let mut targets: Vec<RegionRect> = packer.free_rects().to_vec();
    targets.sort_by(|a, b| {
        b.cell_count()
            .cmp(&a.cell_count())
            .then_with(|| (a.origin().x, a.origin().y).cmp(&(b.origin().x, b.origin().y)))
    });
    for free in targets {
        let candidate = RegionRect::new(free.origin(), footprint);
        if *packer.fits(candidate) && *packer.place(candidate) {
            return Some(PlacedPrefab::new(
                prefab.clone(),
                Anchor::BottomLeft,
                candidate,
            ));
        }
    }
    None
}

pub(super) fn coverage_fraction(covered: CellCount, board_cells: CellCount) -> CoverageFraction {
    if *board_cells <= 0 {
        return CoverageFraction::new(1.0);
    }
    let fraction = *covered as f32 / *board_cells as f32;
    CoverageFraction::new(fraction)
}
