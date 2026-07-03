//! The **fill pass** — the GTW-427 random same-theme fill that pours connective `Fill`
//! prefabs into the free space the GTW-424 player + enemy placement leaves, then pads the
//! remaining dead space with open `default_floor` lanes.
//!
//! This is the SECOND stage of the staged assembler (424 placement -> 427 fill -> 431
//! emit/trigger). It builds on the [`Placement`](super::assembler::Placement) GTW-424
//! produced: it re-derives the packer free space by carving the two placed regions, then
//! draws random same-theme [`Fill`](crate::level::SpawnRole::Fill) prefabs from the
//! registry via [`ProcgenRng`](crate::rng::ProcgenRng) and packs them in — until the
//! coverage [`MinDensityFloor`](super::tuning::MinDensityFloor) is reached OR no more
//! prefab fits (C1/C2 termination).
//!
//! The pass runs three sub-passes against the three OQ-6 knobs ([`ProcgenTuning`]):
//!
//! 1. **`FillLarge`** — place large fill prefabs (footprint area `>=`
//!    [`LargePrefabAreaThreshold`](super::tuning::LargePrefabAreaThreshold)) first, while
//!    the biggest contiguous free rectangles still exist.
//! 2. **Smaller-prefab fill** — place the remaining (sub-threshold) fill prefabs.
//! 3. **Dead-rect scatter** — scatter up to
//!    [`DeadRectScatterCount`](super::tuning::DeadRectScatterCount) micro-pieces into each
//!    `>= 4x4` leftover dead rect, breaking up big empty halls.
//!
//! # The no-fit fallback (RULED — GTW-424 connectivity ruling, user 2026-06-26)
//!
//! When no more prefab fits, the remaining dead space is **padded with `default_floor`
//! (open lanes)** — the playable area is NEVER shrunk. The dead-space-as-floor regions are
//! returned in [`FilledPlacement::dead_space`] so the GTW-431 emit step pours them as
//! `default_floor`; the fill never carves doorways, never walls anything off (OQ-4
//! connectivity-by-construction is preserved by the 1-cell seam every placement still
//! reserves — that seam lattice alone joins every open cell). This is the "chosen no-fit
//! fallback" the AC references.
//!
//! # Determinism
//!
//! The fill draws every prefab choice from the injected [`ProcgenRng`](crate::rng::ProcgenRng)
//! in a FIXED order (the sub-pass order above, and within each pass a deterministic
//! free-rect scan), so the same seed produces an identical fill (the determinism contract).
//! The loop is BOUNDED: every iteration either places a prefab (consuming free space) or
//! ends the pass, so it always terminates (C2 — no infinite loop).

use super::{
    anchor::Anchor,
    assembler::{PlacedPrefab, Placement},
    error::PackingError,
    geometry::{Footprint, Margin, RegionRect},
    packer::{MaxRectsPacker, SplitMode},
    tuning::ProcgenTuning,
};
use crate::{
    level::{GridSize, Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid},
    rng::ProcgenRng,
};

/// The minimum side (in cells) a leftover free rectangle must have on BOTH axes to be a
/// "dead rect" the scatter sub-pass targets (OQ-6: `>= 4x4`).
const DEAD_RECT_MIN_SIDE: i32 = 4;

/// The fully-assembled placement — the GTW-424 [`Placement`] PLUS the GTW-427 fill prefabs
/// and the dead-space-as-`default_floor` regions (C1/C3).
///
/// A named struct (no-bare-types: the filled outcome is a domain value). The GTW-431 emit
/// step pours the spawn placement, every [`fill`](FilledPlacement::fill) prefab, and the
/// [`dead_space`](FilledPlacement::dead_space) regions (as open `default_floor`) into the
/// [`Situation`](crate::situation::Situation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledPlacement {
    /// The GTW-424 player + enemy spawn placement this fill built on.
    placement:  Placement,
    /// The random same-theme `Fill` prefabs the pass packed in, in placement order.
    fill:       Vec<PlacedPrefab>,
    /// The leftover dead-space regions the no-fit fallback PADS with open `default_floor`
    /// (the playable area is never shrunk). The GTW-431 emit step floors these.
    dead_space: Vec<RegionRect>,
}

impl FilledPlacement {
    /// The GTW-424 player + enemy spawn placement.
    #[must_use]
    pub const fn placement(&self) -> &Placement {
        &self.placement
    }

    /// The random same-theme fill prefabs (in placement order).
    #[must_use]
    pub fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

    /// The dead-space regions padded with open `default_floor` (the no-fit fallback —
    /// playable area never shrunk).
    #[must_use]
    pub fn dead_space(&self) -> &[RegionRect] {
        &self.dead_space
    }
}

/// Run the GTW-427 fill pass over a GTW-424 [`Placement`] with the RULED defaults — fill
/// the free space with random same-theme `Fill` prefabs, then pad the remaining dead space
/// with open `default_floor` (C1/C3).
///
/// The RULED-defaults wrapper over [`fill_placement_with`]: the shipped [`SplitMode`], the
/// 1-cell [`Margin::DEFAULT`] seam (OQ-3), and the passed [`ProcgenTuning`] knobs (OQ-6).
///
/// # Errors
///
/// [`PackingError::FootprintDoesNotFit`] if a GTW-424 placed region somehow does not fit
/// the fresh packer (a fail-closed re-guard — the GTW-424 placement already validated fit,
/// so this never fires in practice, but the fill carves those regions before drawing and
/// refuses to proceed from an inconsistent state rather than panic).
pub fn fill_placement(
    placement: Placement,
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    tuning: &ProcgenTuning,
    rng: &mut ProcgenRng,
) -> Result<FilledPlacement, PackingError> {
    fill_placement_with(
        placement,
        registry,
        theme,
        grid_size,
        tuning,
        rng,
        SplitMode::default(),
    )
}

/// The full fill pass with explicit [`SplitMode`] — [`fill_placement`] is the
/// RULED-defaults wrapper.
///
/// Exposed so the A/B comparison and the unit tests can drive the packer's split strategy
/// without changing the shipped default. The 1-cell [`Margin::DEFAULT`] seam (OQ-3) is
/// RULED, so it is NOT a parameter — every fill placement reserves the same 1-cell
/// `default_floor` seam the GTW-424 packer does (no abutting, connectivity-by-construction).
///
/// # Errors
///
/// Same as [`fill_placement`].
pub fn fill_placement_with(
    placement: Placement,
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    tuning: &ProcgenTuning,
    rng: &mut ProcgenRng,
    split: SplitMode,
) -> Result<FilledPlacement, PackingError> {
    let board = RegionRect::board(grid_size);
    let board_cells = board.cell_count().max(1);

    // Re-derive the packer free space by carving the two GTW-424 placed regions, reserving
    // the RULED 1-cell seam (OQ-3). The GTW-424 placement already validated fit, so these
    // always carve cleanly; a false return is an inconsistent state we fail closed on
    // rather than panic.
    let mut packer = MaxRectsPacker::new(board, split, Margin::DEFAULT);
    for region in [placement.player().region(), placement.enemy().region()] {
        if !packer.place(region) {
            return Err(PackingError::FootprintDoesNotFit {
                anchor:    Anchor::BottomLeft,
                footprint: region.footprint(),
                region:    board,
            });
        }
    }

    // The same-theme Fill bucket, split by the large/small area threshold (OQ-6). Both
    // lists are in a deterministic order (largest-first, ties by name) so the RNG draw
    // index is reproducible.
    let mut covered =
        placement.player().region().cell_count() + placement.enemy().region().cell_count();
    let mut fill: Vec<PlacedPrefab> = Vec::new();

    let (large, small) = partition_fill_candidates(registry, theme, tuning);

    // Sub-pass 1 + 2: FillLarge then smaller-prefab fill. Each pass draws random prefabs
    // from its bucket and packs them into the largest fitting free rect until the density
    // floor is reached OR nothing in the bucket fits (bounded — see the loop guard).
    for bucket in [&large, &small] {
        run_fill_pass(
            bucket,
            &mut packer,
            &mut fill,
            &mut covered,
            board_cells,
            *tuning.min_density_floor,
            rng,
        );
    }

    // Sub-pass 3: dead-rect scatter — drop up to k micro-pieces into each >= 4x4 leftover
    // free rect (OQ-6). Uses the SMALL bucket (micro-pieces are small fill prefabs).
    scatter_dead_rects(
        &small,
        &mut packer,
        &mut fill,
        &mut covered,
        tuning.dead_rect_scatter_count_k.count(),
        rng,
    );

    // The no-fit fallback: every remaining free rectangle is padded with open
    // `default_floor` (the playable area is NEVER shrunk). The packer's free list IS the
    // remaining dead space — return it verbatim for the GTW-431 emit step to floor.
    let dead_space: Vec<RegionRect> = packer
        .free_rects()
        .iter()
        .copied()
        .filter(|r| r.is_non_empty())
        .collect();

    Ok(FilledPlacement {
        placement,
        fill,
        dead_space,
    })
}

/// Every same-theme [`Fill`](SpawnRole::Fill) prefab, partitioned into (large, small) by
/// the [`LargePrefabAreaThreshold`](super::tuning::LargePrefabAreaThreshold) — each list in
/// a DETERMINISTIC order (footprint area DESC then name) so the RNG draw index is
/// reproducible (the determinism contract).
fn partition_fill_candidates(
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
fn run_fill_pass(
    bucket: &[Prefab],
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut i64,
    board_cells: i64,
    min_density: f32,
    rng: &mut ProcgenRng,
) {
    if bucket.is_empty() {
        return;
    }
    loop {
        // C1 termination: stop once the coverage fraction reaches the density floor.
        if coverage_fraction(*covered, board_cells) >= min_density {
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
            if !try_any_other(bucket, index, packer, fill, covered) {
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
    skip: usize,
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut i64,
) -> bool {
    for (i, prefab) in bucket.iter().enumerate() {
        if i == skip {
            continue;
        }
        if let Some(placed) = place_in_largest_free(packer, prefab) {
            *covered += placed.region().cell_count();
            fill.push(placed);
            return true;
        }
    }
    false
}

/// Scatter up to `k` small micro-pieces into each `>= 4x4` leftover dead rect (OQ-6
/// sub-pass 3) — break up big empty halls without clogging them.
///
/// Snapshots the dead rects (free rects of `>= 4x4`) first so the scan is over a stable
/// list, then drops up to `k` random small prefabs into the corner of each. BOUNDED: at
/// most `k` placements per dead rect, over a fixed snapshot — always terminates.
fn scatter_dead_rects(
    bucket: &[Prefab],
    packer: &mut MaxRectsPacker,
    fill: &mut Vec<PlacedPrefab>,
    covered: &mut i64,
    k: usize,
    rng: &mut ProcgenRng,
) {
    if bucket.is_empty() || k == 0 {
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
        for _ in 0..k {
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
        if packer.fits(candidate) && packer.place(candidate) {
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
/// loop compares against the [`MinDensityFloor`](super::tuning::MinDensityFloor).
fn coverage_fraction(covered: i64, board_cells: i64) -> f32 {
    if board_cells <= 0 {
        return 1.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "board cell counts are tiny (<= 60*60 = 3600); the f32 conversion is exact for \
                  this range, so the coverage fraction is exact"
    )]
    let fraction = covered as f32 / board_cells as f32;
    fraction
}
