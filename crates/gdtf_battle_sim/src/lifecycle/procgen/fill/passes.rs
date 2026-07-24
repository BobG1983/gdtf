//! The fill placement PRIMITIVES the [`FillCursor`](super::cursor::FillCursor) drives:
//! candidate partition, the largest-free placement, the deterministic other-member probe, the
//! dead-rect count, and the coverage math.
//!
//! GTW-732 lifted the whole-stage loops (`run_fill_pass` / `scatter_dead_rects`) out of here
//! and into the resumable [`FillCursor`](super::cursor::FillCursor), which drives these
//! primitives one placement at a time; this file keeps only the stateless helpers both the
//! cursor and the old loop share.

use bevy::prelude::Deref;

use super::super::{
    anchor::Anchor,
    assembler::PlacedPrefab,
    geometry::{CellCount, Footprint, RegionRect},
    packer::MaxRectsPacker,
    tuning::ProcgenTuning,
};
use crate::level::{Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid};

/// The minimum side (in cells) a leftover free rectangle must have on BOTH axes to be a
/// "dead rect" the scatter sub-pass targets (OQ-6: `>= 4x4`).
const DEAD_RECT_MIN_SIDE: i32 = 4;

/// The **coverage fraction** the fill loop reaches — placed-prefab cells over total
/// board cells — compared against the [`MinDensityFloor`](super::super::tuning::MinDensityFloor)
/// density target.
///
/// A named newtype over `f32` (no-bare-types: a coverage fraction is a domain value,
/// distinct from the tuning [`MinDensityFloor`](super::super::tuning::MinDensityFloor) it is
/// checked against). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(super) struct CoverageFraction(f32);

impl CoverageFraction {
    /// Build a coverage fraction from its value.
    const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

/// An index into a fill-prefab **bucket** — the just-drawn candidate the C2 "try every
/// other member" sweep skips.
///
/// A named newtype over `usize` (no-bare-types: a bucket index is a domain value).
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BucketIndex(usize);

impl BucketIndex {
    /// Wrap a fill-prefab bucket index.
    pub(super) const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// A COUNT of leftover "dead" free rectangles (`>= 4x4` cells) — the population the scatter
/// sub-pass targets (GTW-732).
///
/// A named newtype over `usize` (no-bare-types: a dead-rect population is a domain quantity).
/// Private inner + derived [`Deref`]. The fill cursor multiplies it by the per-rect cap `k` to
/// bound the total scatter slots.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeadRectCount(usize);

impl DeadRectCount {
    /// Wrap a dead-rect count.
    const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// Every same-theme [`Fill`](SpawnRole::Fill) prefab, partitioned into (large, small) by
/// the [`LargePrefabAreaThreshold`](super::super::tuning::LargePrefabAreaThreshold) — each list in
/// a DETERMINISTIC order (footprint area DESC then name) so the RNG draw index is
/// reproducible (the determinism contract).
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

/// Try to place ANY bucket member (other than `skip`) in the largest fitting free rect,
/// scanning the bucket in deterministic order — returning the placed prefab, or [`None`] if
/// none fits (the C2 "no more fits" signal).
///
/// NO RNG draw (a deterministic scan), so the [`FillCursor`](super::cursor::FillCursor)'s
/// per-placement stepping preserves the EXACT draw sequence of the old whole-stage loop: this
/// is the bounded "nothing fits" probe the old `run_fill_pass` ran inline.
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

/// Count the leftover free rectangles of `>= 4x4` cells — the dead rects the scatter sub-pass
/// drops micro-pieces into (OQ-6). The [`FillCursor`](super::cursor::FillCursor) multiplies
/// this by the per-rect cap `k` to bound the total scatter slots (matching the old
/// `scatter_dead_rects` dead-rect snapshot, which only ever used the count).
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

/// Place `prefab`'s footprint at the min-corner of the LARGEST free rectangle it (plus its
/// margin) fits in, committing the placement to `packer` and returning the [`PlacedPrefab`].
///
/// Deterministic: scans free rects largest-first (cell-count DESC, ties by origin) so the
/// same free list always picks the same target rect. Returns `None` if the footprint fits
/// NO free rect (the C2 "no more fits" signal). The placed prefab is recorded at the free
/// rect's min-corner anchored [`BottomLeft`](Anchor::BottomLeft) — fill prefabs have no
/// deployment anchor, so the field names the corner the fragment was packed against and the
/// GTW-431 emit step reads the region's raw origin.
pub(super) fn place_in_largest_free(
    packer: &mut MaxRectsPacker,
    prefab: &Prefab,
) -> Option<PlacedPrefab> {
    let footprint = Footprint::of(prefab.spec().size);
    // Find the largest free rect the (margin-padded) footprint fits in, deterministically.
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

/// The coverage FRACTION (placed cells over board cells) as an `f32` — the value the fill
/// loop compares against the [`MinDensityFloor`](super::super::tuning::MinDensityFloor).
pub(super) fn coverage_fraction(covered: CellCount, board_cells: CellCount) -> CoverageFraction {
    if *board_cells <= 0 {
        return CoverageFraction::new(1.0);
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "board cell counts are tiny (<= 60*60 = 3600); the f32 conversion is exact for \
                  this range, so the coverage fraction is exact"
    )]
    let fraction = *covered as f32 / *board_cells as f32;
    CoverageFraction::new(fraction)
}
