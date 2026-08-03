//! How each storey is treated relative to the active level.

use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::prelude::Level;

use super::super::active_level::{ActiveLevel, ViewMode};

/// How many storeys below active still show as context.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextDepth(u8);

impl ContextDepth {
    /// Build from a storey count.
    #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}

/// Optional onion isolation around the active storey.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolateView {
    /// No isolation; follow [`ViewMode`] alone.
    #[default]
    Off,
    /// Only the active storey plus this many below stay visible.
    On(ContextDepth),
}

/// Combined view + isolate settings used when classifying a storey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoreyViewMode {
    view: ViewMode,
    isolate: IsolateView,
}

impl StoreyViewMode {
    /// Build from view mode and isolate setting.
    #[must_use]
    pub const fn new(view: ViewMode, isolate: IsolateView) -> Self {
        Self { view, isolate }
    }
}

/// Draw treatment for one storey relative to the active level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreyTreatment {
    /// Do not draw this storey.
    Hidden,
    /// Fully lit active storey.
    Active,
    /// Dimmed context below (or around) the active storey.
    ContextBelow(ContextDepth),
}

/// Classify `storey` under the active level and view mode.
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
