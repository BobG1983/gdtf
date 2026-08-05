//! The content catalogs procgen generates a level from.

use bevy::{ecs::system::SystemParam, prelude::*};
#[cfg(feature = "dev_tools")]
use gdtf_battle_sim::procgen::StagedProcgenRegistries;
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::ProcgenTuning,
    terrain::def::TerrainDefRegistry,
};

use super::procgen::ProcgenRegistries;

/// The catalogs procgen generates a level from, each one absent until Load resolves it.
#[derive(SystemParam)]
pub(crate) struct ProcgenContent<'w> {
    prefabs: Option<Res<'w, PrefabRegistry>>,
    themes:  Option<Res<'w, UuidThemeRegistry>>,
    terrain: Option<Res<'w, TerrainDefRegistry>>,
    tuning:  Option<Res<'w, ProcgenTuning>>,
}

impl ProcgenContent<'_> {
    /// The catalogs as one-shot generation reads them, missing ones left absent.
    pub(super) fn registries(&self) -> ProcgenRegistries<'_> {
        ProcgenRegistries {
            prefabs: self.prefabs.as_deref(),
            themes:  self.themes.as_deref(),
            terrain: self.terrain.as_deref(),
            tuning:  self.tuning.as_deref(),
        }
    }

    /// The catalogs a staged run needs, or `None` while any of them is missing.
    #[cfg(feature = "dev_tools")]
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
