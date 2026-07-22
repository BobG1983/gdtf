//! The **unified step primitive** (GTW-732) — a resumable [`ProcgenCursor`] whose one
//! [`step`](ProcgenCursor::step) advances procgen by exactly ONE unit of work (place one
//! prefab, finalize the fill, or emit) and returns which [`ProcgenStage`] that step belonged
//! to.
//!
//! This is the ONE place placement logic lives.
//! [`generate_level`](super::super::emit::generate_level) is a THIN non-interactive driver
//! that loops [`step`](ProcgenCursor::step) to completion; the app-side stepper's
//! [`StagedProcgen`](super::super::staged::StagedProcgen) is the interactive driver that calls
//! [`step`](ProcgenCursor::step) once per `Next`. Because ONE algorithm has two drivers,
//! stepped-to-completion equals looped-to-completion BY CONSTRUCTION for the same seed — never
//! by testing two independently-written implementations against each other.
//!
//! The cursor takes its [`ProcgenRng`] as a `step` PARAMETER (not an owned field) so
//! [`generate_level`] keeps its caller-owned `&mut ProcgenRng` while the interactive driver
//! owns its own stream — both derived from the SAME seed via
//! [`ProcgenRng::from_root`](crate::rng::ProcgenRng::from_root), so the draw sequences match.

use super::{
    super::{
        assembler::{PlacedPrefab, Placement, place_enemy, place_player},
        emit::emit_level,
        error::PackingError,
        fill::{FillCursor, FillStep, FilledPlacement},
        findings::EmittedLevel,
        geometry::{MinPlayerSide, RegionRect},
        packer::{MaxRectsPacker, SplitMode},
        tuning::ProcgenTuning,
    },
    footprint::{PlacedFootprint, PlacementRole},
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    rng::ProcgenRng,
    terrain::def::TerrainDefRegistry,
};

/// Which pipeline stage a `ProcgenCursor::step` belonged to, or is about to run — a named
/// stage identity (no-bare-types: a stepper UI names the stage it is showing, never a bare
/// index/string).
///
/// `ProcgenCursor::stage` returns the stage the NEXT step will run (or [`Done`](Self::Done)
/// once nothing remains); `ProcgenCursor::step`'s `Ok` return is the stage that JUST ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenStage {
    /// The GTW-424 assemble stage (player then enemy placement — two steps).
    Assemble,
    /// The GTW-427 fill stage (one fill prefab per step, then one finalize step).
    Fill,
    /// The GTW-431 emit stage (pour the filled placement into a `Situation` — one step).
    Emit,
    /// Every stage has completed (or the pipeline failed closed) — nothing left to run.
    Done,
}

/// The read-only registries + tuning one `ProcgenCursor::step` borrows for its unit of work
/// — a named borrow-bundle (no-bare-types: grouped domain sources, not a bare tuple of refs).
/// Built fresh at each call site (from a Bevy system's `Res<_>` handles, or a caller's owned
/// registries), never stored: a [`Resource`](bevy::prelude::Resource) must be `'static`, so
/// [`StagedProcgen`](super::super::staged::StagedProcgen) holds none of these across frames.
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

/// The cursor's private per-unit-of-work state (GTW-732). Never exposed directly — callers
/// read it through [`ProcgenCursor`]'s accessors.
enum Phase {
    /// No step has run — the next step places the player.
    Pending,
    /// The player is placed; the next step places the enemy. Carries the packer with the
    /// player region carved, so the enemy picks against the same free space the one-shot
    /// assemble would.
    PlayerPlaced {
        /// The placed player-spawn prefab.
        player: PlacedPrefab,
        /// The packer with the player region already carved.
        packer: MaxRectsPacker,
    },
    /// The assemble stage is done; the fill sub-cursor is running (one placement per step).
    Filling(FillCursor),
    /// Fill is exhausted; the next step emits. Keeps the [`FilledPlacement`] so the schematic
    /// stays total across the emit step and after.
    Filled(FilledPlacement),
    /// Emit is done. Keeps the [`FilledPlacement`] so the schematic still names every placed
    /// footprint after completion (until the drive is torn down).
    Emitted {
        /// The filled placement the emit poured (retained for the schematic).
        filled: FilledPlacement,
        /// The emitted level.
        level:  EmittedLevel,
    },
    /// A step failed closed. Stored (rather than re-attempted) so a repeat step never re-draws
    /// the RNG for a stage that already consumed it once.
    Failed(PackingError),
}

/// A resumable cursor over the procgen pipeline whose one [`step`](Self::step) advances by
/// exactly one unit of work (GTW-732) — the ONE primitive both the non-interactive
/// [`generate_level`](super::super::emit::generate_level) loop and the interactive
/// [`StagedProcgen`](super::super::staged::StagedProcgen) driver share.
pub(in crate::lifecycle::procgen) struct ProcgenCursor {
    /// The theme every stage generates against (fixed for the whole drive).
    theme:           ThemeUuid,
    /// The board size every stage generates against (fixed for the whole drive).
    grid_size:       GridSize,
    /// The packer split strategy (the RULED default for both drivers).
    split:           SplitMode,
    /// The minimum player-spawn side floor (the RULED default for both drivers).
    min_player_side: MinPlayerSide,
    /// The current per-unit-of-work state.
    phase:           Phase,
}

impl ProcgenCursor {
    /// Start a fresh cursor for `theme` + `grid_size` with the RULED defaults
    /// ([`SplitMode::default`], [`MinPlayerSide::DEFAULT`]).
    #[must_use]
    pub(in crate::lifecycle::procgen) fn new(theme: ThemeUuid, grid_size: GridSize) -> Self {
        Self {
            theme,
            grid_size,
            split: SplitMode::default(),
            min_player_side: MinPlayerSide::DEFAULT,
            phase: Phase::Pending,
        }
    }

    /// The stage the NEXT [`step`](Self::step) will run, or [`ProcgenStage::Done`] once the
    /// drive has finished (successfully or via a failed-closed step).
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn stage(&self) -> ProcgenStage {
        match &self.phase {
            Phase::Pending | Phase::PlayerPlaced { .. } => ProcgenStage::Assemble,
            Phase::Filling(_) => ProcgenStage::Fill,
            Phase::Filled(_) => ProcgenStage::Emit,
            Phase::Emitted { .. } | Phase::Failed(_) => ProcgenStage::Done,
        }
    }

    /// Whether the drive has finished — successfully ([`emitted`](Self::emitted) is `Some`)
    /// or via a failed-closed step ([`failure`](Self::failure) is `Some`).
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn is_done(&self) -> bool {
        matches!(self.stage(), ProcgenStage::Done)
    }

    /// The emitted level, once every stage has completed successfully (`None` before then, and
    /// `None` on a failed-closed drive — see [`failure`](Self::failure)).
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn emitted(&self) -> Option<&EmittedLevel> {
        match &self.phase {
            Phase::Emitted { level, .. } => Some(level),
            _ => None,
        }
    }

    /// The error a step failed closed with, if the drive stopped that way (`None` on an
    /// in-progress or successfully-completed drive).
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn failure(&self) -> Option<&PackingError> {
        match &self.phase {
            Phase::Failed(err) => Some(err),
            _ => None,
        }
    }

    /// The board size this cursor generates against.
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    /// Every placement this cursor has landed so far, as schematic footprints (GTW-732) — the
    /// list an app-side overview draws. Grows by one per placement step, and stays total once
    /// the fill finalizes (so the emit step and the finished drive still name every footprint).
    #[must_use]
    pub(in crate::lifecycle::procgen) fn placed_footprints(&self) -> Vec<PlacedFootprint> {
        match &self.phase {
            Phase::Pending | Phase::Failed(_) => Vec::new(),
            Phase::PlayerPlaced { player, .. } => {
                vec![PlacedFootprint::new(PlacementRole::Player, player.region())]
            }
            Phase::Filling(fill) => footprints_from(fill.placement(), fill.fill()),
            Phase::Filled(filled) | Phase::Emitted { filled, .. } => {
                footprints_from(filled.placement(), filled.fill())
            }
        }
    }

    /// Advance by exactly ONE unit of work and return which [`ProcgenStage`] the step ran.
    ///
    /// Uses `std::mem::replace` to take ownership of the current [`Phase`] (never a
    /// `clone`/`unreachable!`), runs the one unit, and stores the next phase.
    ///
    /// Idempotent once finished: stepping again after the drive is [`Done`](ProcgenStage::Done)
    /// touches neither the RNG nor the stored result and returns `Ok(`[`ProcgenStage::Done`]`)`
    /// (a failed-closed drive re-returns the SAME stored [`PackingError`] rather than
    /// re-attempting the step — a retry would re-draw the RNG and desync from the one-shot loop).
    ///
    /// # Errors
    ///
    /// Propagates the [`PackingError`] the assemble or fill step raised (the emit step is
    /// infallible — connectivity is by-construction, GTW-497).
    pub(in crate::lifecycle::procgen) fn step(
        &mut self,
        registries: StagedProcgenRegistries<'_>,
        rng: &mut ProcgenRng,
    ) -> Result<ProcgenStage, PackingError> {
        let phase = std::mem::replace(&mut self.phase, Phase::Pending);
        let (next, outcome) = match phase {
            Phase::Pending => match place_player(
                registries.prefabs,
                self.theme,
                self.grid_size,
                rng,
                self.split,
                self.min_player_side,
            ) {
                Ok((player, packer)) => (
                    Phase::PlayerPlaced { player, packer },
                    Ok(ProcgenStage::Assemble),
                ),
                Err(err) => (Phase::Failed(err.clone()), Err(err)),
            },
            Phase::PlayerPlaced { player, mut packer } => {
                let board = RegionRect::board(self.grid_size);
                match place_enemy(registries.prefabs, self.theme, &player, &mut packer, board) {
                    Ok(enemy) => {
                        let placement = Placement::new(player, enemy);
                        match FillCursor::new(
                            placement,
                            registries.prefabs,
                            self.theme,
                            self.grid_size,
                            registries.tuning,
                            self.split,
                        ) {
                            Ok(fill) => (Phase::Filling(fill), Ok(ProcgenStage::Assemble)),
                            Err(err) => (Phase::Failed(err.clone()), Err(err)),
                        }
                    }
                    Err(err) => (Phase::Failed(err.clone()), Err(err)),
                }
            }
            Phase::Filling(mut fill) => match fill.step(rng) {
                FillStep::Placed => (Phase::Filling(fill), Ok(ProcgenStage::Fill)),
                FillStep::Exhausted => (Phase::Filled(fill.into_filled()), Ok(ProcgenStage::Fill)),
            },
            Phase::Filled(filled) => {
                let level = emit_level(
                    &filled,
                    self.theme,
                    self.grid_size,
                    registries.themes,
                    registries.terrain_defs,
                );
                (Phase::Emitted { filled, level }, Ok(ProcgenStage::Emit))
            }
            Phase::Emitted { filled, level } => {
                (Phase::Emitted { filled, level }, Ok(ProcgenStage::Done))
            }
            Phase::Failed(err) => {
                let repeated = err.clone();
                (Phase::Failed(err), Err(repeated))
            }
        };
        self.phase = next;
        outcome
    }
}

/// Build the schematic footprint list for a placement + its fill — player, enemy, then each
/// fill prefab in placement order (a FIXED order, so the schematic is stable frame to frame).
fn footprints_from(placement: &Placement, fill: &[PlacedPrefab]) -> Vec<PlacedFootprint> {
    let mut out = Vec::with_capacity(2 + fill.len());
    out.push(PlacedFootprint::new(
        PlacementRole::Player,
        placement.player().region(),
    ));
    out.push(PlacedFootprint::new(
        PlacementRole::Enemy,
        placement.enemy().region(),
    ));
    for placed in fill {
        out.push(PlacedFootprint::new(PlacementRole::Fill, placed.region()));
    }
    out
}
