use bevy::prelude::Deref;

use crate::{
    ganger::{Aiming, Direction, Facing, RingSteps, Stance, StanceKind, Tu},
    tu::spend_tu,
    tuning::{StanceChangeTu, TurnTu},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StanceChanged(bool);

impl StanceChanged {
        #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FacingChanged(bool);

impl FacingChanged {
        #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

pub const fn set_aiming(aiming: &mut Aiming, on: Aiming) {
    *aiming = on;
}

pub fn set_stance(
    stance: &mut Stance,
    tu: &mut Tu,
    to: StanceKind,
    cost: &StanceChangeTu,
) -> StanceChanged {
    if **stance == to {
        return StanceChanged::new(false);
    }
    spend_tu(tu, Tu::new(**cost));
    *stance = Stance::new(to);
    StanceChanged::new(true)
}

pub fn set_facing(facing: &mut Facing, tu: &mut Tu, to: Direction, cost: &TurnTu) -> FacingChanged {
    let total = *(**facing).steps_to(to);
    if total == 0 {
        return FacingChanged::new(false);
    }
    let per = **cost;
    let pool_steps = (**tu).checked_div(per).unwrap_or(total);
    let afford = if pool_steps < total {
        pool_steps
    } else {
        total
    };
    if afford == 0 {
        return FacingChanged::new(false);
    }
    let landing = (**facing).rotated_toward(to, RingSteps::new(afford));
    *facing = Facing::new(landing);
    spend_tu(tu, Tu::new(per * afford));
    FacingChanged::new(true)
}
