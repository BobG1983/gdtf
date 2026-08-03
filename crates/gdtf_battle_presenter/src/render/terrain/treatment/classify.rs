use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::prelude::Level;

use super::super::active_level::{ActiveLevel, ViewMode};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextDepth(u8);

impl ContextDepth {
        #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolateView {
            #[default]
    Off,
            On(ContextDepth),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoreyViewMode {
            view:    ViewMode,
        isolate: IsolateView,
}

impl StoreyViewMode {
        #[must_use]
    pub const fn new(view: ViewMode, isolate: IsolateView) -> Self {
        Self { view, isolate }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreyTreatment {
        Hidden,
        Active,
                                        ContextBelow(ContextDepth),
}

#[must_use]
pub fn storey_treatment(
    storey: Level,
    active: ActiveLevel,
    mode: StoreyViewMode,
) -> StoreyTreatment {
    let storey_ix = *storey;
    let active_ix = **active;
    if storey_ix == active_ix {
        return StoreyTreatment::Active;
    }
    match mode.isolate {
        IsolateView::On(onion) => match active_ix.checked_sub(storey_ix) {
            Some(depth) if depth <= *onion => StoreyTreatment::ContextBelow(ContextDepth(depth)),
            _ => StoreyTreatment::Hidden,
        },
        IsolateView::Off => match mode.view {
            ViewMode::DownToActive => match active_ix.checked_sub(storey_ix) {
                Some(depth) => StoreyTreatment::ContextBelow(ContextDepth(depth)),
                None => StoreyTreatment::Hidden,
            },
            ViewMode::FullView => {
                StoreyTreatment::ContextBelow(ContextDepth(storey_ix.abs_diff(active_ix)))
            }
        },
    }
}
