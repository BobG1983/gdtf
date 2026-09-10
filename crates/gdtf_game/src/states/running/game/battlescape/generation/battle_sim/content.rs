//! The content catalogs procgen generates a level from.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{ProcgenTuning, StagedProcgenRegistries},
    terrain::def::TerrainDefRegistry,
};

/// The catalogs procgen generates a level from, each one absent until Load resolves it.
#[derive(SystemParam)]
pub(crate) struct ProcgenContent<'w> {
    prefabs: Option<Res<'w, PrefabRegistry>>,
    themes:  Option<Res<'w, UuidThemeRegistry>>,
    terrain: Option<Res<'w, TerrainDefRegistry>>,
    tuning:  Option<Res<'w, ProcgenTuning>>,
}

impl ProcgenContent<'_> {
    /// The catalogs a staged run needs, or `None` while any of them is missing.
    pub(crate) fn staged<'a>(
        &'a self,
        fallback_tuning: &'a ProcgenTuning,
    ) -> Option<StagedProcgenRegistries<'a>> {
        Some(StagedProcgenRegistries {
            prefabs:      self.prefabs.as_deref()?,
            themes:       self.themes.as_deref()?,
            terrain_defs: self.terrain.as_deref()?,
            tuning:       self.tuning.as_deref().unwrap_or(fallback_tuning),
        })
    }
}
