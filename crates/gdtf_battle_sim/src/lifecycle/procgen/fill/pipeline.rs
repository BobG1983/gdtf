//! The fill-pass orchestration — [`fill_placement`] / [`fill_placement_with`]: a thin loop
//! over the resumable [`FillCursor`](super::cursor::FillCursor) (GTW-732).

use super::{
    super::{assembler::Placement, error::PackingError, packer::SplitMode, tuning::ProcgenTuning},
    cursor::{FillCursor, FillStep},
    outcome::FilledPlacement,
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
/// 1-cell [`Margin::DEFAULT`](super::super::geometry::Margin::DEFAULT) margin (OQ-3), and the
/// passed [`ProcgenTuning`] knobs (OQ-6).
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
/// A thin loop over the resumable `FillCursor`: step it until it
/// reports the fill exhausted, then finalize it into a [`FilledPlacement`] (GTW-732). The
/// cursor is the ONE fill algorithm — a stepped drive and this looped drive place identically.
/// The 1-cell [`Margin::DEFAULT`](super::super::geometry::Margin::DEFAULT) margin (OQ-3) is
/// RULED, so it is NOT a parameter.
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
    let mut cursor = FillCursor::new(placement, registry, theme, grid_size, tuning, split)?;
    while let FillStep::Placed = cursor.step(rng) {}
    Ok(cursor.into_filled())
}
