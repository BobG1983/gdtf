use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    ganger::{Aiming, Facing, Hp, LifeState, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPosition(PositionFacts);

impl DrawnPosition {
        #[must_use]
    pub const fn new(position: PositionFacts) -> Self {
        Self(position)
    }

            #[must_use]
    pub const fn seeded(position: Position) -> Self {
        Self(PositionFacts::new(position))
    }

        #[must_use]
    pub const fn position(&self) -> Position {
        self.0.inner()
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPose(PoseFacts);

impl DrawnPose {
        #[must_use]
    pub const fn new(pose: PoseFacts) -> Self {
        Self(pose)
    }

        #[must_use]
    pub const fn facing(&self) -> Facing {
        self.0.facing
    }

        #[must_use]
    pub const fn stance(&self) -> Stance {
        self.0.stance
    }

        #[must_use]
    pub const fn aiming(&self) -> Aiming {
        self.0.aiming
    }

        #[must_use]
    pub const fn suppressed(&self) -> bool {
        self.0.suppressed.is_suppressed()
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnLife(LifeState);

impl DrawnLife {
        #[must_use]
    pub const fn new(life: LifeState) -> Self {
        Self(life)
    }
}

#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct DrawnVitals(VitalsFacts);

impl DrawnVitals {
        #[must_use]
    pub const fn new(vitals: VitalsFacts) -> Self {
        Self(vitals)
    }

        #[must_use]
    pub const fn tu(&self) -> Tu {
        self.0.tu
    }

        #[must_use]
    pub const fn hp(&self) -> Hp {
        self.0.hp
    }

        #[must_use]
    pub const fn wounds(&self) -> Wounds {
        self.0.wounds
    }

        #[must_use]
    pub const fn inflicted(&self) -> &InflictedWounds {
        &self.0.inflicted
    }

        #[must_use]
    pub const fn injuries(&self) -> &InflictedInjuries {
        &self.0.injuries
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnMagazine(MagazineFacts);

impl DrawnMagazine {
        #[must_use]
    pub const fn new(magazine: MagazineFacts) -> Self {
        Self(magazine)
    }

        #[must_use]
    pub const fn magazine(&self) -> Magazine {
        self.0.inner()
    }
}
