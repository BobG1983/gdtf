//! The authored situation both hosts load, and the resource it resolves into.

use bevy::{
    asset::AssetServer,
    prelude::{Deref, Resource},
};
use gdtf_battle_sim::situation::Situation;

/// Asset path the authored situation is read from.
pub const SITUATION_RON_PATH: &str = "content/situations/skirmish.ron";

/// The situation the load step resolved for the coming battle.
#[derive(Resource, Deref, Clone, Debug)]
pub struct LoadedSituation(Situation);

impl LoadedSituation {
    /// Wrap the situation the load step resolved.
    #[must_use]
    pub const fn new(situation: Situation) -> Self {
        Self(situation)
    }

    /// The situation, borrowed for editing.
    pub const fn situation_mut(&mut self) -> &mut Situation {
        &mut self.0
    }
}

/// Wrap a situation the hot-RON loader resolved.
#[must_use]
pub fn map_loaded_situation(situation: &Situation, _asset_server: &AssetServer) -> LoadedSituation {
    LoadedSituation::new(situation.clone())
}

/// The situation a host falls back to when the authored file does not load.
#[must_use]
pub fn fallback_loaded_situation() -> LoadedSituation {
    LoadedSituation::new(Situation::default())
}
