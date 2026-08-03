use super::{
    super::{assembler::Placement, error::PackingError, packer::SplitMode, tuning::ProcgenTuning},
    cursor::{FillCursor, FillStep},
    outcome::FilledPlacement,
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid},
    rng::ProcgenRng,
};

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
