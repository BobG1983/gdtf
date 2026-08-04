//! Bevy resource driving staged procgen.

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

/// Staged level generation state held as a Bevy resource.
#[derive(Resource)]
pub struct StagedProcgen {
    rng:    ProcgenRng,
    cursor: ProcgenCursor,
}

impl StagedProcgen {
    /// Start generation from a seed, theme, and board size.
    #[must_use]
    pub fn new(seed: BattleSeed, theme: ThemeUuid, grid_size: GridSize) -> Self {
        Self {
            rng:    ProcgenRng::from_root(seed),
            cursor: ProcgenCursor::new(theme, grid_size),
        }
    }

    /// Current stage.
    #[must_use]
    pub const fn stage(&self) -> ProcgenStage {
        self.cursor.stage()
    }

    /// Whether generation finished (success or failure).
    #[must_use]
    pub const fn is_done(&self) -> bool {
        self.cursor.is_done()
    }

    /// Emitted level if generation succeeded.
    #[must_use]
    pub const fn emitted(&self) -> Option<&EmittedLevel> {
        self.cursor.emitted()
    }

    /// Failure if generation failed.
    #[must_use]
    pub const fn failure(&self) -> Option<&PackingError> {
        self.cursor.failure()
    }

    /// Board size.
    #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.cursor.grid_size()
    }

    /// Footprints placed so far.
    #[must_use]
    pub fn placed_footprints(&self) -> Vec<PlacedFootprint> {
        self.cursor.placed_footprints()
    }

    /// Run one procgen stage.
    ///
    /// # Errors
    ///
    /// Returns [`PackingError`] when the current stage cannot place or fill.
    pub fn advance(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
    ) -> Result<ProcgenStage, PackingError> {
        self.cursor.step(registries, &mut self.rng)
    }

    /// Advance until the cursor is done.
    ///
    /// # Errors
    ///
    /// Returns the first [`PackingError`] from [`Self::advance`].
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
