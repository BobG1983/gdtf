//! The **staged, resumable procgen driver** (GTW-655; reworked GTW-732) — the INTERACTIVE
//! driver over the SAME unified step primitive
//! ([`ProcgenCursor`](super::engine::ProcgenCursor)) that
//! [`generate_level`](super::emit::generate_level) loops NON-interactively.
//!
//! This exists so an app-side dev tool (the GTW-655 load-time stepper, GTW-732 reworked to
//! step per PREFAB) can pause between placements and inspect the intermediate state — without
//! the sim depending on the app, the presenter, or Bevy's render/asset machinery.
//! [`StagedProcgen`] is pure sim data: it borrows its registries only for the duration of ONE
//! [`advance`](StagedProcgen::advance) call (never across frames — a Bevy
//! [`Resource`](bevy::prelude::Resource) must be `'static`, so nothing here holds a registry
//! reference longer than one call), and it derives [`Resource`](bevy::prelude::Resource) itself
//! the same way [`PrefabRegistry`](crate::level::PrefabRegistry) /
//! [`ProcgenTuning`](crate::procgen::ProcgenTuning) already do.
//!
//! Because ONE algorithm (the [`ProcgenCursor`](super::engine::ProcgenCursor)) has two drivers
//! — this interactive one and the batch [`generate_level`](super::emit::generate_level) — a
//! stepped-to-completion drive produces an IDENTICAL [`EmittedLevel`] to a one-shot
//! `generate_level` call for the same seed (pinned by this crate's tests), BY CONSTRUCTION.
//! The primitive takes its [`ProcgenRng`] as a step parameter, so this driver owns its own
//! `ProcgenRng::from_root(seed)` while `generate_level` keeps its caller-owned stream — both
//! derived from the same seed, so their draw sequences match.

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

/// A resumable, one-placement-at-a-time driver over the procgen pipeline
/// [`generate_level`](super::emit::generate_level) otherwise runs to completion in one call
/// (GTW-655; GTW-732 reworked to step per prefab).
///
/// Construct with [`new`](Self::new) (derives its OWN internal [`ProcgenRng`] from the
/// injected [`BattleSeed`], exactly as `generate_level`'s caller does), then call
/// [`advance`](Self::advance) repeatedly — each call runs EXACTLY the next unit of work (one
/// prefab placement, the fill finalize, or the emit). A NORMAL (non-stepper) caller can drive
/// it to completion in one go via [`run_to_completion`](Self::run_to_completion) and get an
/// outcome identical to calling `generate_level` directly (the step-equivalence contract); a
/// stepper UI instead calls `advance` once per user action and reads
/// [`stage`](Self::stage) / [`placed_footprints`](Self::placed_footprints) between calls to
/// show the map assembling piece by piece.
#[derive(Resource)]
pub struct StagedProcgen {
    /// The driver's own procgen RNG stream, advanced in place across `advance` calls so a
    /// stepped drive draws identically to `generate_level`'s single-call drive.
    rng:    ProcgenRng,
    /// The unified step primitive both drivers share.
    cursor: ProcgenCursor,
}

impl StagedProcgen {
    /// Start a new staged drive for `theme` + `grid_size`, deriving its RNG from `seed` via
    /// [`ProcgenRng::from_root`] — the SAME derivation `generate_level`'s caller uses, so a
    /// staged drive and a one-shot `generate_level` call for the SAME seed draw identically.
    #[must_use]
    pub fn new(seed: BattleSeed, theme: ThemeUuid, grid_size: GridSize) -> Self {
        Self {
            rng:    ProcgenRng::from_root(seed),
            cursor: ProcgenCursor::new(theme, grid_size),
        }
    }

    /// The stage the NEXT [`advance`](Self::advance) call will run, or [`ProcgenStage::Done`]
    /// once the drive has finished (successfully or via a failed-closed step).
    #[must_use]
    pub const fn stage(&self) -> ProcgenStage {
        self.cursor.stage()
    }

    /// Whether the drive has finished — successfully ([`emitted`](Self::emitted) is `Some`)
    /// or via a failed-closed step ([`failure`](Self::failure) is `Some`).
    #[must_use]
    pub const fn is_done(&self) -> bool {
        self.cursor.is_done()
    }

    /// The emit result, once every stage has completed successfully (`None` before then, and
    /// `None` on a failed-closed drive — see [`failure`](Self::failure)).
    #[must_use]
    pub const fn emitted(&self) -> Option<&EmittedLevel> {
        self.cursor.emitted()
    }

    /// The error a step failed closed with, if the drive stopped that way (`None` on an
    /// in-progress or successfully-completed drive).
    #[must_use]
    pub const fn failure(&self) -> Option<&PackingError> {
        self.cursor.failure()
    }

    /// The board size this drive generates against.
    #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.cursor.grid_size()
    }

    /// Every placement landed so far, as schematic footprints (GTW-732) — what the app-side
    /// egui overview draws, and how a test observes the map assembling one prefab per step.
    #[must_use]
    pub fn placed_footprints(&self) -> Vec<PlacedFootprint> {
        self.cursor.placed_footprints()
    }

    /// Run EXACTLY the next unit of work and return which stage the step ran.
    ///
    /// Idempotent once finished: calling `advance` again after the drive is
    /// [`Done`](ProcgenStage::Done) touches neither the RNG nor the stored result (a
    /// failed-closed drive re-returns the SAME stored [`PackingError`] rather than
    /// re-attempting the step). Delegates to the shared `ProcgenCursor`.
    ///
    /// # Errors
    ///
    /// Propagates the [`PackingError`] the assemble or fill step raised (the emit step is
    /// infallible — connectivity is by-construction, GTW-497).
    pub fn advance(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
    ) -> Result<ProcgenStage, PackingError> {
        self.cursor.step(registries, &mut self.rng)
    }

    /// Drive every remaining step to completion (the "Skip" / normal-path shape): calls
    /// [`advance`](Self::advance) until [`is_done`](Self::is_done), then returns — the SAME
    /// loop [`generate_level`](super::emit::generate_level) runs.
    ///
    /// # Errors
    ///
    /// Propagates the first [`PackingError`] any step raises (the drive stops there, exactly
    /// as `generate_level` does).
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
