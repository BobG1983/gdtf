//! The **staged, resumable procgen driver** (GTW-655) — the SAME three pipeline stages
//! [`generate_level`](super::emit::generate_level) runs to completion in one call (assemble
//! -> fill -> emit), exposed as an explicit state machine a caller can drive ONE STAGE at a
//! time instead of all at once.
//!
//! This exists so an app-side dev tool (the GTW-655 load-time stepper) can pause BETWEEN
//! stages and inspect the intermediate placement — without the sim depending on the app,
//! the presenter, or Bevy's render/asset machinery. [`StagedProcgen`] is pure sim data: it
//! borrows its registries only for the duration of ONE [`advance`](StagedProcgen::advance)
//! call (never across frames — a Bevy [`Resource`](bevy::prelude::Resource) must be
//! `'static`, so nothing here holds a registry reference longer than one call), and it
//! derives [`Resource`](bevy::prelude::Resource) itself the same way
//! [`PrefabRegistry`](crate::level::PrefabRegistry) /
//! [`ProcgenTuning`](crate::procgen::ProcgenTuning) already do — a sim data type doubling as
//! a Bevy resource is an established pattern here (`bevy_ecs` is scaffolding, not a render
//! dependency); the sim still never reads the app or the presenter.
//!
//! [`StagedProcgen::advance`] runs the SAME functions [`generate_level`](super::emit::generate_level)
//! composes ([`assemble_placement_with`](super::assembler::assemble_placement_with) ->
//! [`fill_placement_with`](super::fill::fill_placement_with) ->
//! [`emit_level`](super::emit::emit_level)), with the SAME RULED defaults
//! ([`SplitMode::default`], [`MinPlayerSide::DEFAULT`]), so stepping to completion produces
//! an IDENTICAL [`EmittedLevel`] to calling `generate_level` once for the same seed (pinned
//! by this module's tests) — the driver is an alternate SCHEDULE over the existing pipeline,
//! never a second implementation of it.

use bevy::prelude::Resource;

use super::{
    assembler::{Placement, assemble_placement_with},
    emit::emit_level,
    error::PackingError,
    fill::{FilledPlacement, fill_placement_with},
    findings::EmittedLevel,
    geometry::MinPlayerSide,
    packer::SplitMode,
    tuning::ProcgenTuning,
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    rng::{BattleSeed, ProcgenRng},
    terrain::def::TerrainDefRegistry,
};

/// Which pipeline stage a [`StagedProcgen`] is about to run, or JUST ran — a named stage
/// identity (no-bare-types rule: a stepper UI names the stage it is showing, never a bare
/// index/string).
///
/// [`StagedProcgen::stage`] returns the stage the NEXT [`advance`](StagedProcgen::advance)
/// call will run (or [`Done`](Self::Done) once nothing remains);
/// [`StagedProcgen::advance`]'s `Ok` return is the stage that JUST completed (or
/// [`Done`](Self::Done) when called again after completion — an idempotent no-op).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenStage {
    /// The GTW-424 assemble stage (anchor + opposite placement).
    Assemble,
    /// The GTW-427 fill stage (random same-theme fill + dead-space padding).
    Fill,
    /// The GTW-431 emit stage (pour the filled placement into a `Situation`).
    Emit,
    /// Every stage has completed (or the pipeline failed closed) — nothing left to run.
    Done,
}

/// The read-only registries + tuning one [`StagedProcgen::advance`] call borrows for its ONE
/// stage — a named borrow-bundle (no-bare-types: grouped domain sources, not a bare tuple of
/// refs), mirroring the app-side `ProcgenRegistries` precedent. Built fresh at each call site
/// (typically from a Bevy system's `Res<_>` handles), never stored: a [`Resource`] must be
/// `'static`, so [`StagedProcgen`] itself holds none of these across frames.
#[derive(Clone, Copy)]
pub struct StagedProcgenRegistries<'a> {
    /// The UUID-keyed prefab library the assemble + fill stages place from.
    pub prefabs:      &'a PrefabRegistry,
    /// The UUID-keyed theme registry the emit stage resolves the default floor from.
    pub themes:       &'a UuidThemeRegistry,
    /// The terrain-definition registry the emit stage classifies each placed piece against.
    pub terrain_defs: &'a TerrainDefRegistry,
    /// The live procgen fill tuning (the OQ-6 knobs) the fill stage draws against.
    pub tuning:       &'a ProcgenTuning,
}

/// The driver's private per-stage owned state. Never exposed directly — callers read it
/// through [`StagedProcgen::stage`] / [`placement`](StagedProcgen::placement) /
/// [`filled`](StagedProcgen::filled) / [`emitted`](StagedProcgen::emitted) /
/// [`failure`](StagedProcgen::failure).
enum StageState {
    /// No stage has run yet — the next `advance` runs assemble.
    Pending,
    /// Assemble completed; the next `advance` runs fill.
    Assembled(Placement),
    /// Fill completed; the next `advance` runs emit.
    Filled(FilledPlacement),
    /// Emit completed — the driver is done.
    Emitted(EmittedLevel),
    /// A stage failed closed. Stored (rather than re-attempted) so a repeat `advance` call
    /// never re-draws the RNG for a stage that already consumed it once.
    Failed(PackingError),
}

/// A resumable, one-stage-at-a-time driver over the procgen pipeline
/// [`generate_level`](super::emit::generate_level) otherwise runs to completion in one call
/// (GTW-655).
///
/// Construct with [`new`](Self::new) (derives its OWN internal [`ProcgenRng`] from the
/// injected [`BattleSeed`], exactly as `generate_level`'s caller does), then call
/// [`advance`](Self::advance) repeatedly — each call runs EXACTLY the next stage. A NORMAL
/// (non-stepper) caller can drive it to completion in one go via
/// [`run_to_completion`](Self::run_to_completion) and get an outcome identical to calling
/// `generate_level` directly (the GTW-655 step-equivalence contract, pinned by this module's
/// tests); a stepper UI instead calls `advance` once per user action and reads
/// [`stage`](Self::stage) / [`placement`](Self::placement) / [`filled`](Self::filled) /
/// [`emitted`](Self::emitted) between calls to show what just happened.
#[derive(Resource)]
pub struct StagedProcgen {
    /// The theme every stage generates against (fixed for the whole drive).
    theme:     ThemeUuid,
    /// The board size every stage generates against (fixed for the whole drive).
    grid_size: GridSize,
    /// The driver's own procgen RNG stream, advanced in place across `advance` calls so a
    /// stepped drive draws identically to `generate_level`'s single-call drive.
    rng:       ProcgenRng,
    /// The current per-stage owned state.
    state:     StageState,
}

impl StagedProcgen {
    /// Start a new staged drive for `theme` + `grid_size`, deriving its RNG from `seed` via
    /// [`ProcgenRng::from_root`] — the SAME derivation `generate_level`'s caller uses, so a
    /// staged drive and a one-shot `generate_level` call for the SAME seed draw identically.
    #[must_use]
    pub fn new(seed: BattleSeed, theme: ThemeUuid, grid_size: GridSize) -> Self {
        Self {
            theme,
            grid_size,
            rng: ProcgenRng::from_root(seed),
            state: StageState::Pending,
        }
    }

    /// The stage the NEXT [`advance`](Self::advance) call will run, or [`ProcgenStage::Done`]
    /// once the drive has finished (successfully or via a failed-closed stage).
    #[must_use]
    pub const fn stage(&self) -> ProcgenStage {
        match &self.state {
            StageState::Pending => ProcgenStage::Assemble,
            StageState::Assembled(_) => ProcgenStage::Fill,
            StageState::Filled(_) => ProcgenStage::Emit,
            StageState::Emitted(_) | StageState::Failed(_) => ProcgenStage::Done,
        }
    }

    /// Whether the drive has finished — successfully ([`emitted`](Self::emitted) is `Some`)
    /// or via a failed-closed stage ([`failure`](Self::failure) is `Some`).
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self.stage(), ProcgenStage::Done)
    }

    /// The GTW-424 assemble result, once the assemble stage has completed (`None` before it
    /// runs).
    #[must_use]
    pub const fn placement(&self) -> Option<&Placement> {
        match &self.state {
            StageState::Assembled(placement) => Some(placement),
            StageState::Filled(filled) => Some(filled.placement()),
            StageState::Pending | StageState::Emitted(_) | StageState::Failed(_) => None,
        }
    }

    /// The GTW-427 fill result, once the fill stage has completed (`None` before it runs).
    #[must_use]
    pub const fn filled(&self) -> Option<&FilledPlacement> {
        match &self.state {
            StageState::Filled(filled) => Some(filled),
            StageState::Pending
            | StageState::Assembled(_)
            | StageState::Emitted(_)
            | StageState::Failed(_) => None,
        }
    }

    /// The GTW-431 emit result, once every stage has completed successfully (`None` before
    /// then, and `None` on a failed-closed drive — see [`failure`](Self::failure)).
    #[must_use]
    pub const fn emitted(&self) -> Option<&EmittedLevel> {
        match &self.state {
            StageState::Emitted(emitted) => Some(emitted),
            _ => None,
        }
    }

    /// The error a stage failed closed with, if the drive stopped that way (`None` on an
    /// in-progress or successfully-completed drive).
    #[must_use]
    pub const fn failure(&self) -> Option<&PackingError> {
        match &self.state {
            StageState::Failed(err) => Some(err),
            _ => None,
        }
    }

    /// Run EXACTLY the next stage and return which stage just completed.
    ///
    /// Uses the SAME RULED defaults [`generate_level`](super::emit::generate_level) does
    /// ([`SplitMode::default`], [`MinPlayerSide::DEFAULT`]), so a full stepped drive matches
    /// a one-shot `generate_level` call byte-for-byte for the same seed.
    ///
    /// Idempotent once finished: calling `advance` again after the drive is
    /// [`Done`](ProcgenStage::Done) touches neither the RNG nor the stored result and simply
    /// returns `Ok(`[`ProcgenStage::Done`]`)` again (a failed-closed drive re-returns the SAME
    /// stored [`PackingError`] rather than re-attempting the stage — a retry would re-draw the
    /// RNG and desync from the one-shot pipeline).
    ///
    /// # Errors
    ///
    /// Propagates the [`PackingError`] the assemble or fill stage raised (the emit stage is
    /// infallible — connectivity is by-construction, GTW-497).
    pub fn advance(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
    ) -> Result<ProcgenStage, PackingError> {
        let state = std::mem::replace(&mut self.state, StageState::Pending);
        let (next_state, outcome) = match state {
            StageState::Pending => match assemble_placement_with(
                registries.prefabs,
                self.theme,
                self.grid_size,
                &mut self.rng,
                SplitMode::default(),
                MinPlayerSide::DEFAULT,
            ) {
                Ok(placement) => (StageState::Assembled(placement), Ok(ProcgenStage::Assemble)),
                Err(err) => (StageState::Failed(err.clone()), Err(err)),
            },
            StageState::Assembled(placement) => match fill_placement_with(
                placement,
                registries.prefabs,
                self.theme,
                self.grid_size,
                registries.tuning,
                &mut self.rng,
                SplitMode::default(),
            ) {
                Ok(filled) => (StageState::Filled(filled), Ok(ProcgenStage::Fill)),
                Err(err) => (StageState::Failed(err.clone()), Err(err)),
            },
            StageState::Filled(filled) => {
                let emitted = emit_level(
                    &filled,
                    self.theme,
                    self.grid_size,
                    registries.themes,
                    registries.terrain_defs,
                );
                (StageState::Emitted(emitted), Ok(ProcgenStage::Emit))
            }
            StageState::Emitted(emitted) => (StageState::Emitted(emitted), Ok(ProcgenStage::Done)),
            StageState::Failed(err) => {
                let repeated = err.clone();
                (StageState::Failed(err), Err(repeated))
            }
        };
        self.state = next_state;
        outcome
    }

    /// Drive every remaining stage to completion (the "Skip" / normal-path shape): calls
    /// [`advance`](Self::advance) until [`is_done`](Self::is_done), then returns.
    ///
    /// # Errors
    ///
    /// Propagates the first [`PackingError`] any stage raises (the drive stops there, exactly
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
