//! The shared firing GUARD set: the [`mode_tu_cost`] per-shot TU charge, the
//! [`in_bounds`] grid predicate, the [`FireActor`] read-bundle, and the
//! [`can_fire`] guard the HUD button and the `fire()` act both reason over.

use super::ammo::Magazine;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    metric::{Cell, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    tu::can_spend_tu,
    tuning::CombatTuning,
    weapon::FireModeSpec,
};

/// Round a non-negative `f32` TU charge into the unsigned [`Tu`] inner type
/// (`u8`), clamping into the `u8` range so a wild product can never wrap or lose
/// its sign.
///
/// The mode TU charge is a tuning `f32` product (`TuMax` × `ModeTuPercent` × the
/// aim premium); the TU pool is a `u8`. Rounding is **not** pinned by the design (it
/// is unspecified tuning detail) — this uses round-half-away-from-zero
/// ([`f32::round`]). A value below `0` clamps to `0` and above `u8::MAX` to
/// `u8::MAX`. The clamp + localized `#[expect]` is the crate's guarded-cast idiom
/// (see [`crate::resolve_hit`]'s `round_to_i32` / [`crate::apply_hit`]'s
/// `hp_damage_to_u16`), so no `unwrap`/`expect` is needed.
fn charge_to_u8(charge: f32) -> u8 {
    let rounded = charge.round();
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0.0, u8::MAX] first, so the cast can neither wrap nor lose a sign; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(0.0, f32::from(u8::MAX)) as u8;
    clamped
}

/// The TU charge a single shot in `mode` costs the shooter — the **shared source**
/// both [`can_fire`]'s affordability check and the E4.5 `fire()` debit read.
///
/// The per-shot charge (resolution.md §1: per-`FireMode` TU%; §1a: the aim ×1.5
/// premium): `round(TuMax × ModeTuPercent × aim_premium)`, where `aim_premium` is
/// the tuning [`AimTuPremium`](crate::tuning::AimTuPremium) **only when aiming**
/// (`1.0` hip-fired). Defining it once here keeps the affordability test and the
/// actual debit from ever diverging. Returns a [`Tu`] (the guarded `f32→u8` cast
/// via [`charge_to_u8`] — saturating, no `unwrap`); aiming costs strictly more than
/// hip-fire by the `AimTuPremium` factor (resolution.md §1a, default ×1.5).
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
    Tu::new(charge_to_u8(charge))
}

/// Whether a target `(cell, level)` is **inside** the coarse grid extent — the
/// `can_fire` in-bounds predicate.
///
/// The x/y bound is sourced from [`GRID_WIDTH`](crate::occupancy::GRID_WIDTH) /
/// [`GRID_HEIGHT`](crate::occupancy::GRID_HEIGHT) (the 60-cell ground extent's real
/// home in `occupancy.rs`, a STRUCTURAL grid constant — not a metric constant), and
/// the z bound from [`MAX_LEVELS`](crate::metric::MAX_LEVELS) (the 8-storey home in
/// `metric.rs`): `cell.x ∈ 0..GRID_WIDTH`, `cell.y ∈ 0..GRID_HEIGHT`,
/// `level ∈ 0..MAX_LEVELS`. Negative x/y fail (a [`Cell`] wraps a signed `IVec2`)
/// because the `usize::try_from` of a negative coordinate is `Err` — mirroring
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid)'s own bounds-check choke
/// point, so the firing guard and the occupancy buffer agree on the grid edge.
#[must_use]
pub fn in_bounds(cell: Cell, level: Level) -> bool {
    // Cell coordinates are signed (IVec2); a negative axis is out of bounds and
    // `usize::try_from` rejects it, so no `usize as i32` cast (which would trip
    // `cast_possible_wrap`) is ever needed.
    let Ok(x) = usize::try_from(cell.x) else {
        return false;
    };
    let Ok(y) = usize::try_from(cell.y) else {
        return false;
    };
    x < GRID_WIDTH && y < GRID_HEIGHT && (*level as usize) < MAX_LEVELS as usize
}

/// The shooter read-state [`can_fire`] reasons over — the ganger components a
/// firing decision depends on, bundled into one named record.
///
/// Grouping these five borrowed components keeps [`can_fire`] under clippy's
/// argument-count gate (the [`crate::aim::Shooter`] /
/// [`crate::resolve_coarse::ShotInputs`] bundle precedent), and states the
/// shooter's contribution to the firing guard as one value rather than five loose
/// params. Every field is a borrowed named domain newtype (no bare primitive); the
/// bundle is a transparent borrow record, not itself a wrapped domain scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FireActor<'a> {
    /// The shooter's [`LifeState`] — `can_fire` requires
    /// [`Alive`](crate::ganger::LifeState::Alive) (a Downed/Dead ganger cannot
    /// fire).
    pub life:     &'a LifeState,
    /// The shooter's current [`Tu`] pool — must afford the mode's TU charge.
    pub tu:       &'a Tu,
    /// The shooter's [`TuMax`] — the round-start ceiling the per-shot charge is a
    /// percentage of.
    pub tu_max:   &'a TuMax,
    /// The shooter's [`Aiming`] flag — selects whether the aim TU premium applies
    /// to the charge.
    pub aiming:   &'a Aiming,
    /// The shooter's [`Magazine`] — must hold at least one round.
    pub magazine: &'a Magazine,
}

/// Whether a shooter **can fire** the selected mode at a target — the guard set the
/// HUD fire button and the E4.5 `fire()` act SHARE (resolution.md §9
/// `can_stabilize` precedent: "button and act share one guard set").
///
/// Returns `true` iff ALL hold (AC3): the shooter is
/// [`Alive`](crate::ganger::LifeState::Alive) (AC4 — Downed/Dead fails); affords
/// the mode's TU charge ([`mode_tu_cost`], via E4.0
/// [`can_spend_tu`](crate::tu::can_spend_tu) — AC5, and the aiming case costs
/// strictly more by the [`AimTuPremium`](crate::tuning::AimTuPremium)); has at
/// least one round loaded (AC6 — an empty [`Magazine`] fails); and the target
/// `(target_cell, target_level)` is [`in_bounds`] (AC7).
///
/// **It takes NO `has_los`/visibility input** (AC8): LOS/fog is PLAYER POLICY in
/// the presenter (resolution.md §"What's pure math vs sim": fog "never enters the
/// shared act"), so an alive, affordable, loaded, in-bounds shooter passes here
/// regardless of any LOS state — the presenter applies its own fog gate on top.
#[must_use]
pub fn can_fire(
    actor: &FireActor,
    mode: &FireModeSpec,
    target_cell: Cell,
    target_level: Level,
    tuning: &CombatTuning,
) -> bool {
    *actor.life == LifeState::Alive
        && can_spend_tu(
            actor.tu,
            mode_tu_cost(mode, actor.tu_max, actor.aiming, tuning),
        )
        && !actor.magazine.is_empty()
        && in_bounds(target_cell, target_level)
}
