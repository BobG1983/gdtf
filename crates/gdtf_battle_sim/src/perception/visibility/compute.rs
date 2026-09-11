//! FOV union across observers and accrue into squad fog.

use bevy::platform::collections::HashSet;

use crate::{
    cover::CoverLedger,
    ganger::{Facing, LifeState, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, can_see},
    march::MarchGrids,
    metric::{Cell, CellLevel, CellUnit, Level},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, GridExtent, OccupancyGrid, StairEyeOffset},
    surface::SurfaceGrid,
    tuning::{CombatTuning, ViewRange},
    visibility::SquadVisibility,
};

/// One observer feeding the FOV union.
#[derive(Debug, Clone, Copy)]
pub struct FovObserver<'a> {
    /// Observer position.
    pub position:         &'a Position,
    /// Observer stance.
    pub stance:           &'a Stance,
    /// Observer facing.
    pub facing:           &'a Facing,
    /// Life state (dead observers contribute nothing).
    pub life:             LifeState,
    /// Stair eye height offset.
    pub stair_eye_offset: StairEyeOffset,
}

/// Union of all cells visible to any active observer within view range.
/// Bounded to a disc of radius `view_range` (not full shadowcasting).
#[must_use]
pub fn union_fov(
    observers: &[FovObserver],
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    is_floored: impl Fn(bevy::prelude::Entity) -> bool,
) -> HashSet<CellLevel> {
    let mut visible = HashSet::default();
    let authored = occupancy.authored_level_range();
    for fov in observers {
        if !*fov.life.is_active() {
            continue;
        }
        let observer = Observer {
            position:         fov.position,
            stance:           fov.stance,
            facing:           fov.facing,
            stair_eye_offset: fov.stair_eye_offset,
            peek_offset:      PeekOffset::default(),
        };
        for (level, cell) in disc_cells(fov.position, tuning.view_range, authored) {
            let candidate = CellLevel::new(cell, level);
            let position = Position::new(candidate);
            let stance = Stance::new(StanceKind::Standing);
            let target = Target {
                position: &position,
                stance:   &stance,
            };
            if *can_see(
                &observer,
                &target,
                fov.life,
                tuning.view_range,
                MarchGrids {
                    occupancy,
                    surface,
                    cover,
                },
                tuning,
                &is_floored,
            ) {
                visible.insert(candidate);
            }
        }
    }
    visible
}

fn disc_cells(
    observer: &Position,
    view_range: ViewRange,
    authored: Option<(Level, Level)>,
) -> impl Iterator<Item = (Level, Cell)> {
    let radius = i32::from(*view_range);
    let observer_level = observer.level();
    let (lo, hi) = match authored {
        None => (observer_level, observer_level),
        Some((lo, hi)) => (lo.min(observer_level), hi.max(observer_level)),
    };
    let x_min = (observer.x - radius).max(0);
    let x_max = (observer.x + radius).min(*CellUnit::from(GridExtent::new(GRID_WIDTH)) - 1);
    let y_min = (observer.y - radius).max(0);
    let y_max = (observer.y + radius).min(*CellUnit::from(GridExtent::new(GRID_HEIGHT)) - 1);
    (*lo..=*hi).flat_map(move |level_index| {
        (y_min..=y_max).flat_map(move |y| {
            (x_min..=x_max).map(move |x| (Level::new(level_index), Cell::new(x, y)))
        })
    })
}

/// Fold newly visible cells into the squad's explored set; replace current FOV.
#[must_use]
pub fn accrue(previous: &SquadVisibility, visible_next: HashSet<CellLevel>) -> SquadVisibility {
    let mut explored: HashSet<CellLevel> = previous.explored_cells().copied().collect();
    explored.extend(visible_next.iter().copied());
    SquadVisibility::new(visible_next, explored)
}
