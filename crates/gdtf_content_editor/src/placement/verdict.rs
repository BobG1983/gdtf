use gdtf_battle_sim::{metric::CellLevel, terrain::def::TerrainUuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalReason {
        OutOfBounds,
                SlabSealsLadder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementVerdict {
            Legal {
                        auto_clear: Option<CellLevel>,
    },
        Illegal(IllegalReason),
}

impl PlacementVerdict {
        #[must_use]
    pub const fn legal() -> Self {
        Self::Legal { auto_clear: None }
    }

        #[must_use]
    pub const fn legal_clearing(slot: CellLevel) -> Self {
        Self::Legal {
            auto_clear: Some(slot),
        }
    }

            #[must_use]
    pub const fn is_illegal(&self) -> bool {
        matches!(self, Self::Illegal(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposedPlacement {
        slot: CellLevel,
        tile: TerrainUuid,
}

impl ProposedPlacement {
        #[must_use]
    pub const fn new(slot: CellLevel, tile: TerrainUuid) -> Self {
        Self { slot, tile }
    }

        #[must_use]
    pub const fn slot(&self) -> CellLevel {
        self.slot
    }

        #[must_use]
    pub const fn tile(&self) -> TerrainUuid {
        self.tile
    }
}
