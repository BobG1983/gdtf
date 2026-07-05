//! Shared actor / fire-mode fixtures for the magazine tests — reached by each
//! concern file via `use super::support::*;`.

use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    magazine::{FireActor, Magazine, ReloadTu},
    weapon::{FireModeSpec, Handedness, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

/// An arbitrary per-weapon reload cost the magazine fixtures carry — the magnitude is
/// tunable, so tests never assert it; they assert RELATIONS over the magazine state.
pub(super) const RELOAD_TU: ReloadTu = ReloadTu::new(12);

/// A fire-mode spec with an arbitrary (non-pinned) TU% — the per-mode magnitude
/// is tuning, so tests never assert it; they assert RELATIONS over it.
pub(super) const fn mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A loaded, affordable, alive, aim-flag-controllable actor over borrowed state
/// — the shared fixture the `can_fire` cases vary one field at a time. Wields a
/// `OneHanded` weapon with the uninjured two-hands default, so the GTW-443 hand-count
/// clause always passes here (the hand-count cases use [`hand_actor`] to vary it).
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

/// An alive, affordable, loaded actor with a chosen [`Handedness`] +
/// [`HandsAvailable`] — the GTW-443 hand-count fixture, varying only the two hand fields
/// (everything else passes `can_fire`).
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
