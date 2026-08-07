//! Pure helpers to update pose components and spend TU.

use bevy::prelude::Deref;

use super::cost::{afforded_turn_steps, can_set_stance, stance_tu_cost, turn_tu_cost};
use crate::{
    ganger::{Aiming, Direction, Facing, Stance, StanceKind, Tu},
    tu::spend_tu,
    tuning::{StanceChangeTu, TurnTu},
};

/// Whether stance actually changed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StanceChanged(bool);

impl StanceChanged {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

/// Whether facing actually changed.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FacingChanged(bool);

impl FacingChanged {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

/// Set aiming on or off (no TU cost).
pub const fn set_aiming(aiming: &mut Aiming, on: Aiming) {
    *aiming = on;
}

/// Change stance if different and affordable; spends [`stance_tu_cost`] on success.
pub fn set_stance(
    stance: &mut Stance,
    tu: &mut Tu,
    to: StanceKind,
    cost: &StanceChangeTu,
) -> StanceChanged {
    if !*can_set_stance(stance, to, tu, cost) {
        return StanceChanged::new(false);
    }
    if spend_tu(tu, stance_tu_cost(cost)).is_err() {
        return StanceChanged::new(false);
    }
    *stance = Stance::new(to);
    StanceChanged::new(true)
}

/// Rotate facing toward `to` as far as TU allows; spends [`turn_tu_cost`] for the afforded steps.
pub fn set_facing(facing: &mut Facing, tu: &mut Tu, to: Direction, cost: &TurnTu) -> FacingChanged {
    let from: Direction = **facing;
    let afforded = afforded_turn_steps(from, to, tu, cost);
    if *afforded == 0 {
        return FacingChanged::new(false);
    }
    if spend_tu(tu, turn_tu_cost(afforded, cost)).is_err() {
        return FacingChanged::new(false);
    }
    *facing = Facing::new(from.rotated_toward(to, afforded));
    FacingChanged::new(true)
}
