use bevy::prelude::Deref;

use crate::{
    combatants::ganger::{Aiming, Facing, Hp, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SuppressedNow(bool);

impl SuppressedNow {
            #[must_use]
    pub const fn new(suppressed: bool) -> Self {
        Self(suppressed)
    }

            #[must_use]
    pub const fn is_suppressed(self) -> bool {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoseFacts {
        pub facing:     Facing,
        pub stance:     Stance,
        pub aiming:     Aiming,
        pub suppressed: SuppressedNow,
}

impl PoseFacts {
        #[must_use]
    pub const fn new(
        facing: Facing,
        stance: Stance,
        aiming: Aiming,
        suppressed: SuppressedNow,
    ) -> Self {
        Self {
            facing,
            stance,
            aiming,
            suppressed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VitalsFacts {
        pub tu:        Tu,
        pub hp:        Hp,
        pub wounds:    Wounds,
        pub inflicted: InflictedWounds,
        pub injuries:  InflictedInjuries,
}

impl VitalsFacts {
        #[must_use]
    pub const fn new(
        tu: Tu,
        hp: Hp,
        wounds: Wounds,
        inflicted: InflictedWounds,
        injuries: InflictedInjuries,
    ) -> Self {
        Self {
            tu,
            hp,
            wounds,
            inflicted,
            injuries,
        }
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MagazineFacts(Magazine);

impl MagazineFacts {
        #[must_use]
    pub const fn new(magazine: Magazine) -> Self {
        Self(magazine)
    }

        #[must_use]
    pub const fn inner(self) -> Magazine {
        self.0
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionFacts(Position);

impl PositionFacts {
        #[must_use]
    pub const fn new(position: Position) -> Self {
        Self(position)
    }

        #[must_use]
    pub const fn inner(self) -> Position {
        self.0
    }
}
