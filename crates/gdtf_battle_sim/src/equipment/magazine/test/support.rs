use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    magazine::{FireActor, Magazine, ReloadTu},
    weapon::{FireModeSpec, Handedness, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

pub(super) const RELOAD_TU: ReloadTu = ReloadTu::new(12);

pub(super) const fn mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

pub(super) fn actor<'a>(
    life: &'a LifeState,
    tu: &'a Tu,
    tu_max: &'a TuMax,
    aiming: &'a Aiming,
    magazine: &'a Magazine,
) -> FireActor<'a> {
    FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness: Handedness::OneHanded,
        hands_available: HandsAvailable::default(),
    }
}

pub(super) fn hand_actor<'a>(
    life: &'a LifeState,
    tu: &'a Tu,
    tu_max: &'a TuMax,
    aiming: &'a Aiming,
    magazine: &'a Magazine,
    handedness: Handedness,
    hands_available: HandsAvailable,
) -> FireActor<'a> {
    FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness,
        hands_available,
    }
}
