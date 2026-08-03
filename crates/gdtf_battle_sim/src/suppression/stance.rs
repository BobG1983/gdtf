//! Auto-drop stance toward cover when newly suppressed.

use bevy::prelude::{Added, Query, Res};

use crate::{
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, Position, Stance, StanceKind, Suppressed},
    metric::{Cell, CellLevel},
};

/// Stance kind implied by cover height toward the suppressor.
#[must_use]
pub const fn stance_for_cover_band(band: HeightBand) -> StanceKind {
    match band {
        HeightBand::Low => StanceKind::Prone,
        HeightBand::Mid | HeightBand::High => StanceKind::Crouching,
    }
}

fn cover_cell_toward(unit: &Position, suppressor: &CellLevel) -> Option<CellLevel> {
    let unit_cell = unit.cell();
    let dir = Direction::from_cells(unit_cell, suppressor.cell())?;
    let step = dir.cell_step();
    let toward = Cell::new(unit_cell.x + step.x, unit_cell.y + step.y);
    Some(CellLevel::new(toward, unit.level()))
}

/// When [`Suppressed`] is added, set stance from cover between unit and suppressor.
pub fn suppression_auto_stance(
    mut newly: Query<(&Position, &Suppressed, &mut Stance), Added<Suppressed>>,
    cover: Res<CoverLedger>,
) {
    for (position, suppressed, mut stance) in &mut newly {
        let Some(cover_cell) = cover_cell_toward(position, &suppressed.from) else {
            continue;
        };
        let Some(entry) = cover.peek(&cover_cell) else {
            continue;
        };
        let kind = stance_for_cover_band(entry.height_band);
        *stance = Stance::new(kind);
    }
}
