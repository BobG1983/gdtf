use std::ops::RangeInclusive;

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    metric::MAX_LEVELS,
    prelude::{CellLevel, Level},
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    treatment::{IsolateView, StoreyTreatment, StoreyViewMode, storey_treatment},
};

/// Which storeys the camera draws: active level, view mode, and isolation.
#[derive(SystemParam)]
pub struct DrawnStoreys<'w> {
    active:  Res<'w, ActiveLevel>,
    view:    Res<'w, ViewMode>,
    isolate: Res<'w, IsolateView>,
}

/// Whether the storey band the camera draws changed this tick.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StoreyViewChanged(bool);

impl StoreyViewChanged {
    /// Wrap a change-detection answer.
    const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

impl DrawnStoreys<'_> {
    /// Whether any of the three storey-view resources changed this tick.
    pub(crate) fn changed(&self) -> StoreyViewChanged {
        StoreyViewChanged::new(
            self.active.is_changed() || self.view.is_changed() || self.isolate.is_changed(),
        )
    }

    /// Storeys to draw, lowest first.
    pub fn levels(&self) -> impl Iterator<Item = Level> {
        level_band(drawn_band(
            *self.active,
            StoreyViewMode::new(*self.view, *self.isolate),
        ))
    }
}

pub(super) fn cell_level_in_band(at: CellLevel, band: &RangeInclusive<Level>) -> bool {
    let start = i32::from(**band.start());
    let end = i32::from(**band.end());
    (start..=end).contains(&at.z)
}

pub(super) fn level_band(band: RangeInclusive<Level>) -> impl Iterator<Item = Level> {
    (**band.start()..=**band.end()).map(Level::new)
}

pub(super) fn drawn_band(active: ActiveLevel, mode: StoreyViewMode) -> RangeInclusive<Level> {
    let mut floor = **active;
    let mut ceiling = **active;
    for storey in 0..MAX_LEVELS {
        if storey_treatment(Level::new(storey), active, mode) != StoreyTreatment::Hidden {
            floor = floor.min(storey);
            ceiling = ceiling.max(storey);
        }
    }
    Level::new(floor)..=Level::new(ceiling)
}
