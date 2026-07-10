//! The posture / orientation verb implementations — [`set_aiming`], [`set_stance`],
//! [`set_facing`]. See the module docs (`super`) for the design rationale.

use bevy::prelude::Deref;

use crate::{
    ganger::{Aiming, Direction, Facing, RingSteps, Stance, StanceKind, Tu},
    tu::spend_tu,
    tuning::{StanceChangeTu, TurnTu},
};

/// Whether a [`set_stance`] call actually changed the ganger's posture (and charged TU).
///
/// `true` means the stance moved to a new posture and [`StanceChangeTu`] was spent;
/// `false` means the ganger already held the requested stance (a no-op, no charge). A
/// distinct posture-change verdict, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StanceChanged(bool);

impl StanceChanged {
    /// Build the stance-change verdict.
    #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

/// Whether a [`set_facing`] call actually turned the ganger (and charged TU).
///
/// `true` means the facing rotated at least one 45° step and [`TurnTu`] was spent;
/// `false` means the ganger already faced the requested direction, or the pool could not
/// afford even one step (a no-op, no charge). A distinct turn verdict, not a bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FacingChanged(bool);

impl FacingChanged {
    /// Build the facing-change verdict.
    #[must_use]
    pub const fn new(changed: bool) -> Self {
        Self(changed)
    }
}

/// Set a ganger's [`Aiming`] flag — **charges no TU**.
///
/// A pure setter of the aim-mode flag: `*aiming = on`. Toggling aim is
/// NOT a costed action (`docs/combat/combat.md` L34's action list does not name it);
/// the aim cost is the **fire-time ×1.5 shot-cost premium** (`docs/combat/resolution.md`
/// §1a, the [`crate::tuning::AimTuPremium`] leaf), charged when the shot fires, not per
/// toggle. So no [`Tu`] is touched here — this verb takes no TU pool at all.
pub const fn set_aiming(aiming: &mut Aiming, on: Aiming) {
    *aiming = on;
}

/// Change a ganger's [`Stance`] to `to`, charging [`StanceChangeTu`] **only on a real
/// change** — returns whether the stance changed.
///
/// If the ganger already holds posture `to`, this is a **no-op**: the stance is
/// unchanged and **no TU is spent** (you don't pay to not move), and it returns
/// `false`. Otherwise it spends `cost` from `tu` via [`crate::tu::spend_tu`] (saturating
/// — it floors at `0`, never underflows) and sets the new stance, returning `true`.
///
/// The charge is grounded in `docs/combat/combat.md` L34 ("kneel" costs TUs) and
/// `docs/combat/resolution.md` §"What's tunable" (stance-change TU); the magnitude is
/// the [`StanceChangeTu`] tuning leaf, not a hardcoded constant.
pub fn set_stance(
    stance: &mut Stance,
    tu: &mut Tu,
    to: StanceKind,
    cost: &StanceChangeTu,
) -> StanceChanged {
    if **stance == to {
        // Re-asserting the posture the ganger already holds — a no-op, no charge.
        return StanceChanged::new(false);
    }
    spend_tu(tu, Tu::new(**cost));
    *stance = Stance::new(to);
    StanceChanged::new(true)
}

/// Turn a ganger toward [`Direction`] `to`, charging **one [`TurnTu`] per 45deg step**
/// and turning only as far as the [`Tu`] pool affords — the **PARTIAL-TURN** model.
/// Returns whether the facing changed.
///
/// `cost` is the per-step price (`TurnTu` per 45deg step). The full short-way rotation is
/// `total = (**facing).steps_to(to)` steps; the ganger turns the `afford` whole steps its
/// pool pays for — `afford = min(total, pool / per)` (treating `per == 0` as fully
/// affordable) — landing on `to` when it can pay for the whole turn, otherwise **partway**
/// at `(**facing).rotated_toward(to, afford)`. The charge is **exact**: `per * afford`,
/// which by construction never exceeds the held pool (it still goes through the one
/// saturating [`crate::tu::spend_tu`] path).
///
/// Two no-turn cases return `false` and charge **nothing**: re-asserting the held facing
/// (`steps_to == 0` — you don't pay to not turn) and a pool that cannot afford even one
/// step (`afford == 0`, i.e. `pool < per`). Otherwise it sets the (possibly intermediate)
/// facing and returns `true`.
///
/// Turning **costs** TU: `docs/combat/combat.md` L34 affirmatively lists "turn" among the
/// actions that cost TUs; the per-step magnitude is the [`TurnTu`] tuning leaf
/// (value-agnostic, USER DECISION: `1` per step). NOT a free toggle.
pub fn set_facing(facing: &mut Facing, tu: &mut Tu, to: Direction, cost: &TurnTu) -> FacingChanged {
    // The full short-way rotation, in whole 45deg steps. Zero == already facing `to`.
    let total = *(**facing).steps_to(to);
    if total == 0 {
        // Re-asserting the facing the ganger already holds — a no-op, no charge.
        return FacingChanged::new(false);
    }
    let per = **cost;
    // How many WHOLE steps the pool pays for, capped at the full rotation. A zero
    // per-step cost is treated as fully affordable: `checked_div` yields `None` on a
    // zero divisor, which we read as "the whole rotation is free" (guards divide-by-zero).
    let pool_steps = (**tu).checked_div(per).unwrap_or(total);
    let afford = if pool_steps < total {
        pool_steps
    } else {
        total
    };
    if afford == 0 {
        // The pool cannot afford even one step (pool < per) — no turn, no charge.
        return FacingChanged::new(false);
    }
    // Compute the landing facing BEFORE mutating; lands at `to` iff afford == total.
    let landing = (**facing).rotated_toward(to, RingSteps::new(afford));
    *facing = Facing::new(landing);
    // Exact charge: per * afford never exceeds the pool by construction.
    spend_tu(tu, Tu::new(per * afford));
    FacingChanged::new(true)
}
