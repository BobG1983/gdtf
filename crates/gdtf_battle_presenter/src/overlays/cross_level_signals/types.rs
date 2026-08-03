use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelDelta(i8);

impl LevelDelta {
        #[must_use]
    pub const fn new(delta: i8) -> Self {
        Self(delta)
    }

            #[must_use]
    pub const fn is_above(self) -> bool {
        self.0 > 0
    }

            #[must_use]
    pub const fn magnitude(self) -> u8 {
        self.0.unsigned_abs()
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreatCount(u8);

impl ThreatCount {
        pub const ONE: Self = Self(1);

            #[must_use]
    pub const fn incremented(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossLevelBadgeKind {
                Threat {
                delta: LevelDelta,
                count: ThreatCount,
    },
        DropDepth {
                storeys: StoreysFallen,
    },
        ConnectorDelta {
                        delta: LevelDelta,
    },
}

pub const BADGE_CAP_PER_CELL: usize = 3;

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossLevelSignals {
    per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>,
}

impl CrossLevelSignals {
            #[must_use]
    pub(super) const fn build(per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>) -> Self {
        Self { per_cell }
    }

            #[must_use]
    pub fn badges_at(&self, cell: Cell) -> &[CrossLevelBadgeKind] {
        self.per_cell.get(&cell).map_or(&[], Vec::as_slice)
    }

        pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        self.per_cell.keys().copied()
    }
}
