use std::ops::RangeInclusive;

use gdtf_battle_sim::{
    metric::MAX_LEVELS,
    prelude::{CellLevel, Level},
};

use super::{
    active_level::ActiveLevel,
    treatment::{StoreyTreatment, StoreyViewMode, storey_treatment},
};

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
