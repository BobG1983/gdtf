//! Running totals of injury effects on a combatant.

use bevy::prelude::{Component, Deref};

use super::{BleedAmount, GainedInjury, HandsAvailable, MovementCostFactor, StatDelta, StatTarget};
use crate::{
    armor::BodyPart,
    effects::injuries::{ApplyInjuryEffect, LedgerAccumulators},
};

/// Sum of all deltas applied to one stat.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatDeltaSum(i16);

impl StatDeltaSum {
    /// Build from a raw sum.
    #[must_use]
    pub const fn new(sum: i16) -> Self {
        Self(sum)
    }

    /// Add a delta, saturating.
    #[must_use]
    pub const fn add(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_add(delta.raw() as i16))
    }

    /// Subtract a delta, saturating.
    #[must_use]
    pub const fn subtract(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_sub(delta.raw() as i16))
    }
}

/// Per-stat delta totals for every tracked stat.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatDeltaLedger([StatDeltaSum; StatTarget::COUNT]);

impl Default for StatDeltaLedger {
    fn default() -> Self {
        Self([StatDeltaSum::default(); StatTarget::COUNT])
    }
}

impl StatDeltaLedger {
    /// Add a delta to one stat.
    pub const fn add_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].add(delta);
    }

    /// Remove a previously applied delta.
    pub const fn remove_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].subtract(delta);
    }

    /// Current total for one stat.
    #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.0[stat.index()]
    }
}

/// Total bleed points currently applied.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BleedAfflicted(u16);

impl BleedAfflicted {
    /// Build from a raw total.
    #[must_use]
    pub const fn new(total: u16) -> Self {
        Self(total)
    }

    /// Add more bleed.
    #[must_use]
    pub const fn accumulate(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_add(amount.raw() as u16))
    }

    /// Reduce bleed (e.g. after stabilize).
    #[must_use]
    pub const fn relieve(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_sub(amount.raw() as u16))
    }
}

/// All active injuries and the accumulated effects they produce.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct InflictedInjuries {
    gained:   Vec<GainedInjury>,
    deltas:   StatDeltaLedger,
    bleed:    BleedAfflicted,
    movement: MovementCostFactor,
}

impl InflictedInjuries {
    /// Apply a new injury and fold its effects into the ledgers.
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

    /// Injuries currently on this combatant.
    #[must_use]
    pub fn gained(&self) -> &[GainedInjury] {
        &self.gained
    }

    /// Net delta for one stat.
    #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.deltas.delta_for(stat)
    }

    /// Current bleed total.
    #[must_use]
    pub const fn bleed(&self) -> BleedAfflicted {
        self.bleed
    }

    /// Movement cost multiplier from injuries.
    #[must_use]
    pub const fn movement_cost_factor(&self) -> MovementCostFactor {
        self.movement
    }

    /// How many hands remain usable after arm-disabling injuries.
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
