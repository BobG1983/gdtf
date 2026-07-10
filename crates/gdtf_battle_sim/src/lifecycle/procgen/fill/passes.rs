//! The fill sub-pass engines + placement primitives: candidate partition, the
//! bounded fill loop, the dead-rect scatter, the largest-free placement, and the
//! coverage math.

use bevy::prelude::Deref;

use super::super::{
    anchor::Anchor,
    assembler::PlacedPrefab,
    geometry::{CellCount, Footprint, RegionRect},
    packer::MaxRectsPacker,
    tuning::{MinDensityFloor, ProcgenTuning, ScatterCount},
};
use crate::{
    level::{Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid},
    rng::ProcgenRng,
};

/// The minimum side (in cells) a leftover free rectangle must have on BOTH axes to be a
/// "dead rect" the scatter sub-pass targets (OQ-6: `>= 4x4`).
const DEAD_RECT_MIN_SIDE: i32 = 4;

/// The **coverage fraction** the fill loop reaches — placed-prefab cells over total
/// board cells — compared against the [`MinDensityFloor`] density target.
///
/// A named newtype over `f32` (no-bare-types: a coverage fraction is a domain value,
/// distinct from the tuning [`MinDensityFloor`] it is checked against). Private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, PartialOrd)]
struct CoverageFraction(f32);

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
struct BucketIndex(usize);

impl BucketIndex {
    /// Wrap a fill-prefab bucket index.
    const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Whether the C2 sweep **placed** a prefab — `false` means no bucket member fit
/// anywhere (the pass is exhausted).
///
/// A named newtype over `bool` (no-bare-types: a placement verdict is a domain fact).
/// Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct PrefabPlaced(bool);

impl PrefabPlaced {
    /// Build a placement verdict from its boolean state.
    const fn new(placed: bool) -> Self {
        Self(placed)
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

/// Run one fill sub-pass: while coverage is below the density floor, draw a random prefab
/// from `bucket` and place it in the largest fitting free rect; stop when the floor is
/// reached OR no prefab in the bucket fits anywhere (C1/C2 termination).
///
/// BOUNDED: each iteration either places a prefab (strictly consuming free space and
/// advancing `covered`) or finds nothing fits and breaks — so the loop always terminates.
pub(super) fn run_fill_pass(
    bucket: &[Prefab],
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut CellCount,
    board_cells: CellCount,
    min_density: MinDensityFloor,
    rng: &mut ProcgenRng,
) {
    if bucket.is_empty() {
        return;
    }
    loop {
        // C1 termination: stop once the coverage fraction reaches the density floor.
        if *coverage_fraction(*covered, board_cells) >= *min_density {
            return;
        }
        // Draw a random prefab from the bucket (the one RNG draw per attempt) and try to
        // place it in the largest fitting free rect.
        let index = rng.random_range(0..bucket.len());
        let Some(prefab) = bucket.get(index) else {
            return;
        };
        let Some(placed) = place_in_largest_free(packer, prefab) else {
            // C2 termination: this draw did not fit. Try every OTHER bucket member once
            // (deterministic scan); if NONE fits, the pass is exhausted — break.
            if !*try_any_other(bucket, BucketIndex::new(index), packer, fill, covered) {
                return;
            }
            continue;
        };
        *covered += placed.region().cell_count();
        fill.push(placed);
    }
}

/// Try to place ANY bucket member (other than the just-failed `skip` index) in the largest
/// fitting free rect, scanning the bucket in deterministic order. Returns whether one was
/// placed — if not, the pass is exhausted (C2: no more prefab fits).
///
/// This is the bounded "nothing fits" probe: it makes a single deterministic sweep, so it
/// adds at most `bucket.len()` work per outer iteration and cannot loop forever.
fn try_any_other(
    bucket: &[Prefab],
    skip: BucketIndex,
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut CellCount,
) -> PrefabPlaced {
    for (i, prefab) in bucket.iter().enumerate() {
        if i == *skip {
            continue;
        }
        if let Some(placed) = place_in_largest_free(packer, prefab) {
            *covered += placed.region().cell_count();
            fill.push(placed);
            return PrefabPlaced::new(true);
        }
    }
    PrefabPlaced::new(false)
}

/// Scatter up to `k` small micro-pieces into each `>= 4x4` leftover dead rect (OQ-6
/// sub-pass 3) — break up big empty halls without clogging them.
///
/// Snapshots the dead rects (free rects of `>= 4x4`) first so the scan is over a stable
/// list, then drops up to `k` random small prefabs into the corner of each. BOUNDED: at
/// most `k` placements per dead rect, over a fixed snapshot — always terminates.
pub(super) fn scatter_dead_rects(
    bucket: &[Prefab],
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut CellCount,
    k: ScatterCount,
    rng: &mut ProcgenRng,
) {
    if bucket.is_empty() || *k == 0 {
        return;
    }
    // Snapshot the dead rects (>= 4x4) so we iterate a stable list while the packer mutates.
    let dead_rects: Vec<RegionRect> = packer
        .free_rects()
        .iter()
        .copied()
        .filter(|r| {
            r.footprint().width() >= DEAD_RECT_MIN_SIDE
                && r.footprint().height() >= DEAD_RECT_MIN_SIDE
        })
        .collect();

    for _rect in dead_rects {
        for _ in 0..*k {
            // Draw a random small micro-piece and try to drop it into the largest fitting
            // free rect (the snapshot rect may have been split by a prior scatter, so we
            // re-query the live free list rather than reusing the stale snapshot rect).
            let index = rng.random_range(0..bucket.len());
            let Some(prefab) = bucket.get(index) else {
                continue;
            };
            let Some(placed) = place_in_largest_free(packer, prefab) else {
                // Nothing more fits anywhere — this dead rect (and any after it) cannot
                // take a scatter piece. Stop scattering entirely.
                return;
            };
            *covered += placed.region().cell_count();
            fill.push(placed);
        }
    }
}

/// Place `prefab`'s footprint at the min-corner of the LARGEST free rectangle it (plus its
/// seam) fits in, committing the placement to `packer` and returning the [`PlacedPrefab`].
///
/// Deterministic: scans free rects largest-first (cell-count DESC, ties by origin) so the
/// same free list always picks the same target rect. Returns `None` if the footprint fits
/// NO free rect (the C2 "no more fits" signal). The placed prefab is recorded at the free
/// rect's min-corner anchored [`BottomLeft`](Anchor::BottomLeft) — fill prefabs have no
/// deployment anchor, so the field names the corner the fragment was packed against and the
/// GTW-431 emit step reads the region's raw origin.
fn place_in_largest_free(packer: &mut MaxRectsPacker, prefab: &Prefab) -> Option<PlacedPrefab> {
    let footprint = Footprint::of(prefab.spec().size);
    // Find the largest free rect the (seam-padded) footprint fits in, deterministically.
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
fn coverage_fraction(covered: CellCount, board_cells: CellCount) -> CoverageFraction {
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
