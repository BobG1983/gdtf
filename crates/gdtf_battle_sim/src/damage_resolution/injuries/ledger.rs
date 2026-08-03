use bevy::prelude::{Component, Deref};

use super::{BleedAmount, GainedInjury, HandsAvailable, MovementCostFactor, StatDelta, StatTarget};
use crate::{
    armor::BodyPart,
    effects::injuries::{ApplyInjuryEffect, LedgerAccumulators},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatDeltaSum(i16);

impl StatDeltaSum {
            #[must_use]
    pub const fn new(sum: i16) -> Self {
        Self(sum)
    }

            #[must_use]
    pub const fn add(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_add(delta.raw() as i16))
    }

                #[must_use]
    pub const fn subtract(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_sub(delta.raw() as i16))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatDeltaLedger([StatDeltaSum; StatTarget::COUNT]);

impl Default for StatDeltaLedger {
    fn default() -> Self {
        Self([StatDeltaSum::default(); StatTarget::COUNT])
    }
}

impl StatDeltaLedger {
        pub const fn add_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].add(delta);
    }

            pub const fn remove_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].subtract(delta);
    }

            #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.0[stat.index()]
    }
}

#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BleedAfflicted(u16);

impl BleedAfflicted {
            #[must_use]
    pub const fn new(total: u16) -> Self {
        Self(total)
    }

        #[must_use]
    pub const fn accumulate(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_add(amount.raw() as u16))
    }

            #[must_use]
    pub const fn relieve(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_sub(amount.raw() as u16))
    }
}

#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct InflictedInjuries {
            gained:   Vec<GainedInjury>,
        deltas:   StatDeltaLedger,
        bleed:    BleedAfflicted,
                        movement: MovementCostFactor,
}

impl InflictedInjuries {
                                                                    pub fn gain(&mut self, record: GainedInjury) {
        let mut accumulators = LedgerAccumulators {
            deltas:   &mut self.deltas,
            bleed:    &mut self.bleed,
            movement: &mut self.movement,
        };
        for effect in &record.effects {
            effect.fold_on_gain(&mut accumulators);
        }
        self.gained.push(record);
    }

            #[must_use]
    pub fn gained(&self) -> &[GainedInjury] {
        &self.gained
    }

            #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.deltas.delta_for(stat)
    }

        #[must_use]
    pub const fn bleed(&self) -> BleedAfflicted {
        self.bleed
    }

                                                    #[must_use]
    pub const fn movement_cost_factor(&self) -> MovementCostFactor {
        self.movement
    }

                                                                                #[must_use]
    pub fn hands_available(&self) -> HandsAvailable {
        let mut left_disabled = false;
        let mut right_disabled = false;
        for record in &self.gained {
            let disables = record.effects.iter().any(|e| *e.disables_hand());
            if !disables {
                continue;
            }
            match record.part {
                BodyPart::LeftArm => left_disabled = true,
                BodyPart::RightArm => right_disabled = true,
                BodyPart::Head | BodyPart::Torso | BodyPart::LeftLeg | BodyPart::RightLeg => {}
            }
        }
        let disabled = u8::from(left_disabled) + u8::from(right_disabled);
        HandsAvailable::new(HandsAvailable::MAX - disabled)
    }
}
