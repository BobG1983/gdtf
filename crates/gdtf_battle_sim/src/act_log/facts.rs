//! Snapshots of pose, vitals, magazine, and position for change detection.

use bevy::prelude::Deref;

use crate::{
    combatants::ganger::{Aiming, Facing, Hp, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

/// Whether the actor is suppressed right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SuppressedNow(bool);

impl SuppressedNow {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(suppressed: bool) -> Self {
        Self(suppressed)
    }

    /// True if suppressed.
    #[must_use]
    pub const fn is_suppressed(self) -> bool {
        self.0
    }
}

/// Facing, stance, aiming, and suppression snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoseFacts {
    /// Facing direction.
    pub facing: Facing,
    /// Stance.
    pub stance: Stance,
    /// Aim state.
    pub aiming: Aiming,
    /// Suppression flag.
    pub suppressed: SuppressedNow,
}

impl PoseFacts {
    /// Build pose facts.
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

/// TU, HP, wounds, and injury lists.
#[derive(Debug, Clone, PartialEq)]
pub struct VitalsFacts {
    /// Time units remaining.
    pub tu: Tu,
    /// Hit points.
    pub hp: Hp,
    /// Wound capacity / track.
    pub wounds: Wounds,
    /// Inflicted wound list.
    pub inflicted: InflictedWounds,
    /// Inflicted injuries.
    pub injuries: InflictedInjuries,
}

impl VitalsFacts {
    /// Build vitals facts.
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

/// Magazine snapshot.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MagazineFacts(Magazine);

impl MagazineFacts {
    /// Wrap a magazine.
    #[must_use]
    pub const fn new(magazine: Magazine) -> Self {
        Self(magazine)
    }

    /// Inner magazine.
    #[must_use]
    pub const fn inner(self) -> Magazine {
        self.0
    }
}

/// Position snapshot.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionFacts(Position);

impl PositionFacts {
    /// Wrap a position.
    #[must_use]
    pub const fn new(position: Position) -> Self {
        Self(position)
    }

    /// Inner position.
    #[must_use]
    pub const fn inner(self) -> Position {
        self.0
    }
}
