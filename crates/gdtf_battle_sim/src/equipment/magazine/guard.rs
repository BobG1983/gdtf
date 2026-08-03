use bevy::prelude::Deref;

use super::ammo::Magazine;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    metric::{Cell, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    tu::can_spend_tu,
    tuning::CombatTuning,
    weapon::{FireModeSpec, Handedness},
};

/// `u8::MAX`. The clamp + localized `#[expect]` is the crate's guarded-cast idiom
fn charge_to_u8(charge: TuCharge) -> Tu {
    let rounded = charge.round();
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0.0, u8::MAX] first, so the cast can neither wrap nor lose a sign; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(0.0, f32::from(u8::MAX)) as u8;
    Tu::new(clamped)
}

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct TuCharge(f32);

impl TuCharge {
        #[must_use]
    const fn new(charge: f32) -> Self {
        Self(charge)
    }
}

#[must_use]
pub fn mode_tu_cost(
    mode: &FireModeSpec,
    tu_max: &TuMax,
    aiming: &Aiming,
    tuning: &CombatTuning,
) -> Tu {
    let aim_premium = if **aiming {
        *tuning.cone_stability.aim_mode.tu_premium
    } else {
        1.0
    };
    let charge = f32::from(**tu_max) * *mode.tu_percent * aim_premium;
    charge_to_u8(TuCharge::new(charge))
}

#[must_use]
pub fn in_bounds(cell: Cell, level: Level) -> InBounds {
    let Ok(x) = usize::try_from(cell.x) else {
        return InBounds(false);
    };
    let Ok(y) = usize::try_from(cell.y) else {
        return InBounds(false);
    };
    InBounds(x < GRID_WIDTH && y < GRID_HEIGHT && (*level as usize) < MAX_LEVELS as usize)
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InBounds(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FireActor<'a> {
                pub life:            &'a LifeState,
        pub tu:              &'a Tu,
            pub tu_max:          &'a TuMax,
            pub aiming:          &'a Aiming,
        pub magazine:        &'a Magazine,
                pub handedness:      Handedness,
            pub hands_available: HandsAvailable,
}

#[must_use]
pub fn can_fire(
    actor: &FireActor,
    mode: &FireModeSpec,
    target_cell: Cell,
    target_level: Level,
    tuning: &CombatTuning,
) -> CanFire {
    CanFire(
        *actor.life == LifeState::Alive
            && *can_spend_tu(
                actor.tu,
                mode_tu_cost(mode, actor.tu_max, actor.aiming, tuning),
            )
            && !*actor.magazine.is_empty()
            && *in_bounds(target_cell, target_level)
            && *has_enough_hands(actor.handedness, actor.hands_available),
    )
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanFire(bool);

fn has_enough_hands(handedness: Handedness, hands: HandsAvailable) -> HandsSufficient {
    let needed = match handedness {
        Handedness::OneHanded => 1,
        Handedness::TwoHanded => HandsAvailable::MAX,
    };
    HandsSufficient(*hands >= needed)
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct HandsSufficient(bool);
