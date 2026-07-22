//! The **fill cursor driver** (GTW-732) — the [`FillCursor`] value + its stepping algorithm.
//!
//! It preserves the old whole-stage fill's exact behaviour — the same three sub-passes (large,
//! small, dead-rect scatter) in the same order, drawing the same [`ProcgenRng`] indices at the
//! same points — so a stepped drive and the looped `fill_placement_with` (now a thin
//! `while let Placed = c.step(rng) {}` loop) place IDENTICALLY. The per-`step` yield absorbs
//! zero-placement sub-pass transitions so a `Next` always yields one prefab (or the single
//! exhaustion boundary), never a dead step.

use std::mem;

use super::{
    super::{
        super::{
            anchor::Anchor,
            assembler::{PlacedPrefab, Placement},
            error::PackingError,
            geometry::{CellCount, Margin, RegionRect},
            packer::{MaxRectsPacker, SplitMode},
            tuning::{MaxCoverageCap, MinDensityFloor, ProcgenTuning, ScatterCount},
        },
        outcome::FilledPlacement,
        passes::{
            BucketIndex, coverage_fraction, dead_rect_count, partition_fill_candidates,
            place_in_largest_free, try_any_other_once,
        },
    },
    state::{BucketKind, FillPass, FillStep, ScatterSlots, ScatterState, SubPassStep},
};
use crate::{
    level::{GridSize, Prefab, PrefabRegistry, ThemeUuid},
    rng::ProcgenRng,
};

/// A resumable cursor over the GTW-427 fill pass whose one [`step`](Self::step) places exactly
/// one prefab (GTW-732) — the state that was local to `fill_placement_with` lifted into a value
/// that survives across single-placement steps.
pub(in crate::lifecycle::procgen) struct FillCursor {
    /// The GTW-424 player + enemy placement this fill builds on.
    placement:   Placement,
    /// The packer's free-rect list, mutated per placement.
    packer:      MaxRectsPacker,
    /// The running placed-cell coverage (player + enemy + every fill placement so far).
    covered:     CellCount,
    /// The board's total cell count (clamped `>= 1`), the coverage-fraction denominator.
    board_cells: CellCount,
    /// The accumulated fill placements, in placement order (the schematic + emit read this).
    fill:        Vec<PlacedPrefab>,
    /// The large fill-prefab bucket (deterministic order — largest first, ties by name).
    large:       Vec<Prefab>,
    /// The small fill-prefab bucket (deterministic order).
    small:       Vec<Prefab>,
    /// The minimum coverage fraction the large/small passes fill toward.
    floor:       MinDensityFloor,
    /// The maximum coverage fraction the fill must not exceed — stops EVERY sub-pass early once
    /// reached, even where more prefabs would still fit (GTW-767).
    cap:         MaxCoverageCap,
    /// The per-dead-rect scatter cap `k`.
    scatter_k:   ScatterCount,
    /// The current sub-pass.
    pass:        FillPass,
}

impl FillCursor {
    /// Build a fill cursor over a GTW-424 [`Placement`] — the EXACT prologue the old
    /// `fill_placement_with` ran: rebuild the packer, carve the player then enemy regions
    /// (fail-closed re-guard), seed the coverage, and partition the same-theme fill candidates
    /// into (large, small). Rebuilding the packer (not carrying the assemble packer) matches the
    /// old fill line-for-line — the free-rect lists are provably identical.
    ///
    /// # Errors
    ///
    /// [`PackingError::FootprintDoesNotFit`] if a GTW-424 placed region does not carve cleanly
    /// (a fail-closed re-guard the GTW-424 placement already validated — never fires in
    /// practice, but the cursor refuses to proceed from an inconsistent state rather than panic).
    pub(in crate::lifecycle::procgen) fn new(
        placement: Placement,
        registry: &PrefabRegistry,
        theme: ThemeUuid,
        grid_size: GridSize,
        tuning: &ProcgenTuning,
        split: SplitMode,
    ) -> Result<Self, PackingError> {
        let board = RegionRect::board(grid_size);
        let board_cells = board.cell_count().max(CellCount::new(1));

        let mut packer = MaxRectsPacker::new(board, split, Margin::DEFAULT);
        for region in [placement.player().region(), placement.enemy().region()] {
            if !*packer.place(region) {
                return Err(PackingError::FootprintDoesNotFit {
                    anchor:    Anchor::BottomLeft,
                    footprint: region.footprint(),
                    region:    board,
                });
            }
        }

        let covered =
            placement.player().region().cell_count() + placement.enemy().region().cell_count();
        let (large, small) = partition_fill_candidates(registry, theme, tuning);

        Ok(Self {
            placement,
            packer,
            covered,
            board_cells,
            fill: Vec::new(),
            large,
            small,
            floor: tuning.min_density_floor,
            cap: tuning.max_coverage_cap,
            scatter_k: tuning.dead_rect_scatter_count_k.count(),
            pass: FillPass::Large,
        })
    }

    /// The GTW-424 placement this fill builds on.
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn placement(&self) -> &Placement {
        &self.placement
    }

    /// The fill placements landed so far (in placement order).
    #[must_use]
    pub(in crate::lifecycle::procgen) fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

    /// Advance the fill by exactly one placement, or report the fill exhausted (GTW-732).
    ///
    /// The loop ABSORBS zero-placement sub-pass transitions (Large -> Small -> Scatter ->
    /// Exhausted) so a `Next` always yields one prefab (or the single exhaustion boundary),
    /// never a step that does nothing. The draw order is unchanged from the old whole-stage
    /// fill: each sub-pass draws at the same points, in the same order.
    pub(in crate::lifecycle::procgen) fn step(&mut self, rng: &mut ProcgenRng) -> FillStep {
        loop {
            match mem::replace(&mut self.pass, FillPass::Exhausted) {
                FillPass::Large => match self.bucket_step(BucketKind::Large, rng) {
                    SubPassStep::Placed => {
                        self.pass = FillPass::Large;
                        return FillStep::Placed;
                    }
                    SubPassStep::Done => self.pass = FillPass::Small,
                },
                FillPass::Small => match self.bucket_step(BucketKind::Small, rng) {
                    SubPassStep::Placed => {
                        self.pass = FillPass::Small;
                        return FillStep::Placed;
                    }
                    SubPassStep::Done => self.pass = self.begin_scatter(),
                },
                FillPass::Scatter(mut scatter) => match self.scatter_step(&mut scatter, rng) {
                    SubPassStep::Placed => {
                        self.pass = FillPass::Scatter(scatter);
                        return FillStep::Placed;
                    }
                    SubPassStep::Done => self.pass = FillPass::Exhausted,
                },
                FillPass::Exhausted => {
                    self.pass = FillPass::Exhausted;
                    return FillStep::Exhausted;
                }
            }
        }
    }

    /// One iteration of the old `run_fill_pass` body: if the bucket is empty or the density
    /// floor is reached, the pass is [`Done`](SubPassStep::Done) (no draw); otherwise draw one
    /// random prefab (the ONE draw per attempt) and place it — on a non-fit, scan every OTHER
    /// bucket member once (deterministic, NO draw) and place the first that fits, else the pass
    /// is exhausted.
    fn bucket_step(&mut self, kind: BucketKind, rng: &mut ProcgenRng) -> SubPassStep {
        let Self {
            large,
            small,
            packer,
            covered,
            fill,
            board_cells,
            floor,
            cap,
            ..
        } = self;
        // Disjoint field borrows: the chosen bucket slice + `&mut packer` / `covered` / `fill`
        // are different fields, so the borrow checker admits them together.
        let bucket: &[Prefab] = match kind {
            BucketKind::Large => large.as_slice(),
            BucketKind::Small => small.as_slice(),
        };
        if bucket.is_empty() {
            return SubPassStep::Done;
        }
        // Termination: stop once the coverage fraction reaches the density floor (C1) OR the
        // GTW-767 max coverage cap — either bound, whichever comes first, ends the pass with no
        // draw (so the cap check consumes no RNG, preserving determinism under a fixed cap).
        let coverage = coverage_fraction(*covered, *board_cells);
        if *coverage >= **floor || *coverage >= **cap {
            return SubPassStep::Done;
        }
        // The ONE draw per attempt.
        let index = rng.random_range(0..bucket.len());
        let Some(prefab) = bucket.get(index) else {
            return SubPassStep::Done;
        };
        if let Some(placed) = place_in_largest_free(packer, prefab) {
            *covered += placed.region().cell_count();
            fill.push(placed);
            return SubPassStep::Placed;
        }
        // C2: this draw did not fit. Try every OTHER member once (deterministic, no draw).
        match try_any_other_once(bucket, BucketIndex::new(index), packer) {
            Some(placed) => {
                *covered += placed.region().cell_count();
                fill.push(placed);
                SubPassStep::Placed
            }
            None => SubPassStep::Done,
        }
    }

    /// Begin the dead-rect scatter sub-pass: snapshot the `>= 4x4` dead-rect COUNT times the
    /// per-rect cap `k` into the remaining slots (the only thing the old scatter used from its
    /// snapshot). An empty small bucket, a zero cap, or no dead rects yields
    /// [`FillPass::Exhausted`] (nothing to scatter).
    fn begin_scatter(&self) -> FillPass {
        if self.small.is_empty() || *self.scatter_k == 0 {
            return FillPass::Exhausted;
        }
        // GTW-767: if the large/small passes already reached the coverage cap, do not scatter at
        // all — the scatter sub-pass is coverage-blind by itself and would otherwise overshoot
        // the cap.
        if *coverage_fraction(self.covered, self.board_cells) >= *self.cap {
            return FillPass::Exhausted;
        }
        let slots = *dead_rect_count(&self.packer) * *self.scatter_k;
        if slots == 0 {
            return FillPass::Exhausted;
        }
        FillPass::Scatter(ScatterState {
            slots_remaining: ScatterSlots::new(slots),
        })
    }

    /// One scatter slot: if no slots remain, the sub-pass is [`Done`](SubPassStep::Done) (no
    /// draw); otherwise draw one small micro-piece (the ONE draw), decrement, and drop it into
    /// the largest fitting free rect — on a non-fit, the whole scatter stops (mirrors the old
    /// `return`).
    fn scatter_step(&mut self, scatter: &mut ScatterState, rng: &mut ProcgenRng) -> SubPassStep {
        if *scatter.slots_remaining == 0 {
            return SubPassStep::Done;
        }
        // GTW-767: stop scattering the moment coverage reaches the cap, BEFORE the draw — a
        // cap-driven `Done` consumes no RNG, so a fixed seed + cap yields the same level.
        if *coverage_fraction(self.covered, self.board_cells) >= *self.cap {
            return SubPassStep::Done;
        }
        let Self {
            small,
            packer,
            covered,
            fill,
            ..
        } = self;
        if small.is_empty() {
            return SubPassStep::Done;
        }
        let index = rng.random_range(0..small.len());
        scatter.slots_remaining = ScatterSlots::new(*scatter.slots_remaining - 1);
        let Some(prefab) = small.get(index) else {
            return SubPassStep::Done;
        };
        match place_in_largest_free(packer, prefab) {
            Some(placed) => {
                *covered += placed.region().cell_count();
                fill.push(placed);
                SubPassStep::Placed
            }
            None => SubPassStep::Done,
        }
    }

    /// Consume the cursor into a [`FilledPlacement`]: collect the packer's non-empty free rects
    /// as the dead space (padded with `default_floor` by the emit step) and pair it with the
    /// placement + fill.
    pub(in crate::lifecycle::procgen) fn into_filled(self) -> FilledPlacement {
        let dead_space: Vec<RegionRect> = self
            .packer
            .free_rects()
            .iter()
            .copied()
            .filter(|r| *r.is_non_empty())
            .collect();
        FilledPlacement::new(self.placement, self.fill, dead_space)
    }
}
