use bevy::prelude::Resource;

use super::{
    engine::{PlacedFootprint, ProcgenCursor, ProcgenStage, StagedProcgenRegistries},
    error::PackingError,
    findings::EmittedLevel,
};
use crate::{
    level::{GridSize, ThemeUuid},
    rng::{BattleSeed, ProcgenRng},
};

#[derive(Resource)]
pub struct StagedProcgen {
            rng:    ProcgenRng,
        cursor: ProcgenCursor,
}

impl StagedProcgen {
                #[must_use]
    pub fn new(seed: BattleSeed, theme: ThemeUuid, grid_size: GridSize) -> Self {
        Self {
            rng:    ProcgenRng::from_root(seed),
            cursor: ProcgenCursor::new(theme, grid_size),
        }
    }

            #[must_use]
    pub const fn stage(&self) -> ProcgenStage {
        self.cursor.stage()
    }

            #[must_use]
    pub const fn is_done(&self) -> bool {
        self.cursor.is_done()
    }

            #[must_use]
    pub const fn emitted(&self) -> Option<&EmittedLevel> {
        self.cursor.emitted()
    }

            #[must_use]
    pub const fn failure(&self) -> Option<&PackingError> {
        self.cursor.failure()
    }

        #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.cursor.grid_size()
    }

            #[must_use]
    pub fn placed_footprints(&self) -> Vec<PlacedFootprint> {
        self.cursor.placed_footprints()
    }

                                                pub fn advance(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
    ) -> Result<ProcgenStage, PackingError> {
        self.cursor.step(registries, &mut self.rng)
    }

                                    pub fn run_to_completion(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
    ) -> Result<(), PackingError> {
        while !self.is_done() {
            self.advance(registries)?;
        }
        Ok(())
    }
}
