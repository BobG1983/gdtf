//! Staged procgen cursor: assemble → fill → emit.

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

/// High-level stage of level generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenStage {
    /// Placing player and enemy spawns.
    Assemble,
    /// Filling free space with content prefabs.
    Fill,
    /// Emitting the situation.
    Emit,
    /// Finished (success or failure).
    Done,
}

/// Catalogs needed for one staged step.
#[derive(Clone, Copy)]
pub struct StagedProcgenRegistries<'a> {
    /// Prefab catalog.
    pub prefabs: &'a PrefabRegistry,
    /// Theme catalog.
    pub themes: &'a UuidThemeRegistry,
    /// Terrain piece catalog.
    pub terrain_defs: &'a TerrainDefRegistry,
    /// Density and scatter knobs.
    pub tuning: &'a ProcgenTuning,
}

enum Phase {
    Pending,
    PlayerPlaced {
        player: PlacedPrefab,
        packer: MaxRectsPacker,
    },
    Filling(FillCursor),
    Filled(FilledPlacement),
    Emitted {
        filled: FilledPlacement,
        level: EmittedLevel,
    },
    Failed(PackingError),
}

pub(in crate::lifecycle::procgen) struct ProcgenCursor {
    theme: ThemeUuid,
    grid_size: GridSize,
    split: SplitMode,
    min_player_side: MinPlayerSide,
    phase: Phase,
}

impl ProcgenCursor {
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

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn stage(&self) -> ProcgenStage {
        match &self.phase {
            Phase::Pending | Phase::PlayerPlaced { .. } => ProcgenStage::Assemble,
            Phase::Filling(_) => ProcgenStage::Fill,
            Phase::Filled(_) => ProcgenStage::Emit,
            Phase::Emitted { .. } | Phase::Failed(_) => ProcgenStage::Done,
        }
    }

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn is_done(&self) -> bool {
        matches!(self.stage(), ProcgenStage::Done)
    }

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn emitted(&self) -> Option<&EmittedLevel> {
        match &self.phase {
            Phase::Emitted { level, .. } => Some(level),
            _ => None,
        }
    }

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn failure(&self) -> Option<&PackingError> {
        match &self.phase {
            Phase::Failed(err) => Some(err),
            _ => None,
        }
    }

    #[must_use]
    pub(in crate::lifecycle::procgen) const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

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
