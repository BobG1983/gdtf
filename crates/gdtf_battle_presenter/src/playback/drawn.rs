//! View-side mirrors of sim state driven by the act log.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    ganger::{Aiming, Facing, Hp, LifeState, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

/// Drawn cell position for a ganger.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPosition(PositionFacts);

impl DrawnPosition {
    /// Build from act-log position facts.
    #[must_use]
    pub const fn new(position: PositionFacts) -> Self {
        Self(position)
    }

    /// Seed from live sim position at battle start.
    #[must_use]
    pub const fn seeded(position: Position) -> Self {
        Self(PositionFacts::new(position))
    }

    /// Inner sim position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.0.inner()
    }
}

/// Drawn facing, stance, aim, and suppression.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPose(PoseFacts);

impl DrawnPose {
    /// Build from act-log pose facts.
    #[must_use]
    pub const fn new(pose: PoseFacts) -> Self {
        Self(pose)
    }

    /// Facing direction.
    #[must_use]
    pub const fn facing(&self) -> Facing {
        self.0.facing
    }

    /// Stance.
    #[must_use]
    pub const fn stance(&self) -> Stance {
        self.0.stance
    }

    /// Aiming flag.
    #[must_use]
    pub const fn aiming(&self) -> Aiming {
        self.0.aiming
    }

    /// Whether the ganger is suppressed.
    #[must_use]
    pub const fn suppressed(&self) -> bool {
        self.0.suppressed.is_suppressed()
    }
}

/// Drawn life state.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnLife(LifeState);

impl DrawnLife {
    /// Build from a life state.
    #[must_use]
    pub const fn new(life: LifeState) -> Self {
        Self(life)
    }
}

/// Drawn TU, HP, wounds, and injuries.
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct DrawnVitals(VitalsFacts);

impl DrawnVitals {
    /// Build from act-log vitals facts.
    #[must_use]
    pub const fn new(vitals: VitalsFacts) -> Self {
        Self(vitals)
    }

    /// Remaining TU.
    #[must_use]
    pub const fn tu(&self) -> Tu {
        self.0.tu
    }

    /// Current HP.
    #[must_use]
    pub const fn hp(&self) -> Hp {
        self.0.hp
    }

    /// Wound count.
    #[must_use]
    pub const fn wounds(&self) -> Wounds {
        self.0.wounds
    }

    /// Inflicted wound list.
    #[must_use]
    pub const fn inflicted(&self) -> &InflictedWounds {
        &self.0.inflicted
    }

    /// Inflicted injury list.
    #[must_use]
    pub const fn injuries(&self) -> &InflictedInjuries {
        &self.0.injuries
    }
}

/// Drawn magazine state on a weapon entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnMagazine(MagazineFacts);

impl DrawnMagazine {
    /// Build from act-log magazine facts.
    #[must_use]
    pub const fn new(magazine: MagazineFacts) -> Self {
        Self(magazine)
    }

    /// Inner magazine value.
    #[must_use]
    pub const fn magazine(&self) -> Magazine {
        self.0.inner()
    }
}
