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

pub(in crate::lifecycle::procgen) struct FillCursor {
    placement:   Placement,
    packer:      MaxRectsPacker,
    covered:     CellCount,
    board_cells: CellCount,
    fill:        Vec<PlacedPrefab>,
    large:       Vec<Prefab>,
    small:       Vec<Prefab>,
    floor:       MinDensityFloor,
    cap:         MaxCoverageCap,
    scatter_k:   ScatterCount,
    pass:        FillPass,
}

impl FillCursor {
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

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn placement(&self) -> &Placement {
        &self.placement
    }

    #[must_use]
    pub(in crate::lifecycle::procgen) fn fill(&self) -> &[PlacedPrefab] {
        &self.fill
    }

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
        let bucket: &[Prefab] = match kind {
            BucketKind::Large => large.as_slice(),
            BucketKind::Small => small.as_slice(),
        };
        if bucket.is_empty() {
            return SubPassStep::Done;
        }
        let coverage = coverage_fraction(*covered, *board_cells);
        if *coverage >= **floor || *coverage >= **cap {
            return SubPassStep::Done;
        }
        let index = rng.random_range(0..bucket.len());
        let Some(prefab) = bucket.get(index) else {
            return SubPassStep::Done;
        };
        if let Some(placed) = place_in_largest_free(packer, prefab) {
            *covered += placed.region().cell_count();
            fill.push(placed);
            return SubPassStep::Placed;
        }
        match try_any_other_once(bucket, BucketIndex::new(index), packer) {
            Some(placed) => {
                *covered += placed.region().cell_count();
                fill.push(placed);
                SubPassStep::Placed
            }
            None => SubPassStep::Done,
        }
    }

    fn begin_scatter(&self) -> FillPass {
        if self.small.is_empty() || *self.scatter_k == 0 {
            return FillPass::Exhausted;
        }
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

    fn scatter_step(&mut self, scatter: &mut ScatterState, rng: &mut ProcgenRng) -> SubPassStep {
        if *scatter.slots_remaining == 0 {
            return SubPassStep::Done;
        }
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
