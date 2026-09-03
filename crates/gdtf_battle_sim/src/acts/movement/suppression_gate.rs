//! A suppressed mover walks only by getting farther out, behind cover or out of sight.

use bevy::prelude::{Deref, Entity};

use super::sight::SightWorld;
use crate::{
    cover::CoverLedger,
    ganger::{Direction, Facing, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Sighted, Target, has_los_skipping},
    march::SkippedOccupant,
    metric::{Cell, CellDistance, CellLevel},
};

/// The stance the probe aims at, since the cell the fire came from records none.
const SHOT_CELL_STANCE: Stance = Stance::new(StanceKind::Standing);

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct EndsBehindCover(bool);

impl EndsBehindCover {
    const fn new(behind_cover: bool) -> Self {
        Self(behind_cover)
    }
}

/// Whether a suppressed mover is allowed to take this step.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SuppressedMoveLegal(bool);

impl SuppressedMoveLegal {
    /// Wrap whether the step is allowed.
    #[must_use]
    pub(crate) const fn new(legal: bool) -> Self {
        Self(legal)
    }
}

/// Who the mover is, where it stands and how it is posed while its break-away is judged.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BreakAwayMover<'a> {
    entity: Entity,
    start:  &'a Position,
    stance: &'a Stance,
    facing: &'a Facing,
}

impl<'a> BreakAwayMover<'a> {
    /// Build from the mover, where its walk starts, and the pose it holds.
    #[must_use]
    pub(crate) const fn new(
        entity: Entity,
        start: &'a Position,
        stance: &'a Stance,
        facing: &'a Facing,
    ) -> Self {
        Self {
            entity,
            start,
            stance,
            facing,
        }
    }
}

fn chebyshev_xy(a: &CellLevel, b: &CellLevel) -> CellDistance {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    CellDistance::new(dx.max(dy))
}

fn ends_behind_cover(
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
) -> EndsBehindCover {
    let dest_cell = dest.cell();
    let suppressor_cell = suppressor.cell();
    let Some(dir) = Direction::from_cells(dest_cell, suppressor_cell) else {
        return EndsBehindCover::new(false);
    };
    let step = dir.cell_step();
    let toward = Cell::new(dest_cell.x + step.x, dest_cell.y + step.y);
    EndsBehindCover::new(cover.peek(&CellLevel::new(toward, dest.level())).is_some())
}

// Whether the mover would still see the cell the fire came from once it is on `dest`, at its own stance.
fn sees_shot_cell<F: Fn(Entity) -> bool>(
    mover: &BreakAwayMover<'_>,
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
    sight: &SightWorld<'_, F>,
) -> Sighted {
    let from = Position::new(*dest);
    let at = Position::new(*suppressor);
    let observer = Observer {
        position:         &from,
        stance:           mover.stance,
        facing:           mover.facing,
        stair_eye_offset: sight.occupancy().stair_eye_offset_at(dest),
        peek_offset:      PeekOffset::default(),
    };
    let shot_cell = Target {
        position: &at,
        stance:   &SHOT_CELL_STANCE,
    };
    // The mover's own cell is crossed as if empty; a dead body still occludes at the floor band.
    has_los_skipping(
        &observer,
        &shot_cell,
        SkippedOccupant::new(mover.entity),
        sight.march(cover),
        sight.tuning(),
        sight.is_dead(),
    )
}

/// Legal when dest is farther from the suppressor and either takes cover or loses sight of it.
#[must_use]
pub(crate) fn suppressed_move_legal<F: Fn(Entity) -> bool>(
    mover: &BreakAwayMover<'_>,
    dest: &CellLevel,
    suppressor: &CellLevel,
    cover: &CoverLedger,
    sight: &SightWorld<'_, F>,
) -> SuppressedMoveLegal {
    if chebyshev_xy(dest, suppressor) <= chebyshev_xy(mover.start, suppressor) {
        return SuppressedMoveLegal::new(false);
    }
    let broke_away = *ends_behind_cover(dest, suppressor, cover)
        || !*sees_shot_cell(mover, dest, suppressor, cover, sight);
    SuppressedMoveLegal::new(broke_away)
}
