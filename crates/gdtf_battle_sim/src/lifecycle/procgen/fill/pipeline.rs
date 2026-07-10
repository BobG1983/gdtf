//! The fill-pass orchestration — [`fill_placement`] / [`fill_placement_with`]: carve
//! the placed regions, run the three sub-passes, pad the dead space.

use super::{
    super::{
        anchor::Anchor,
        assembler::{PlacedPrefab, Placement},
        error::PackingError,
        geometry::{Margin, RegionRect},
        packer::{MaxRectsPacker, SplitMode},
        tuning::ProcgenTuning,
    },
    outcome::FilledPlacement,
    passes::{partition_fill_candidates, run_fill_pass, scatter_dead_rects},
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid},
    rng::ProcgenRng,
};

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
            tuning.min_density_floor,
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
