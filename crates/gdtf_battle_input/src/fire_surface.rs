//! The FIRE decision (GTW-227 / GTW-238): the shared `can_fire` guard set folded into
//! a reusable helper the unified left-click surface ([`crate::selection::left_click_act`])
//! consults as the FIRST rung of its precedence chain.
//!
//! There is NO fire BUTTON — fire is left-click-on-an-ENEMY-target with a fire mode
//! selected (the 222c action bar carries stance / aim / mode / level only; an explicit
//! fire button is GTW-11). This module assembles the shooter's
//! [`FireActor`](gdtf_battle_sim::FireActor) from its QUERIED components and runs the
//! SHARED [`can_fire`](gdtf_battle_sim::can_fire) guard set (the SAME validation the
//! 222c button and the sim `fire()` consult — resolution.md §9 "button and act share
//! one guard set"); when it passes it yields the [`FireRequested`] the unified system
//! pushes as an [`ActIntent::Fire`](crate::ActIntent::Fire). The ONE
//! [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits the carried message.
//!
//! Until GTW-238 this was a standalone `fire_on_click` system racing
//! `select_on_click` via ordering; GTW-238 REPLACED that two-system race with one
//! disambiguated left-click decision, so the FIRE logic now lives here as a helper the
//! unified system calls — no separate fire system runs.
//!
//! `can_fire` takes NO LOS input — LOS / fog is presenter policy, out of scope here
//! (resolution.md §"What's pure math vs sim").

use bevy::prelude::*;
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, FireActor, Level, LifeState, Magazine, Tu, TuMax, acts::FireRequested,
    can_fire, tuning::CombatTuning,
};

use crate::SelectedFireMode;

/// The shooter components the FIRE branch queries to assemble a
/// [`FireActor`](gdtf_battle_sim::FireActor) for the shared `can_fire` guard.
///
/// The exact five reads `FireActor` borrows (`magazine.rs`): the shooter's
/// [`LifeState`], current [`Tu`], [`TuMax`], [`Aiming`] flag, and [`Magazine`]. A
/// `QueryData` tuple of borrowed sim components — framework plumbing, not a domain
/// scalar. Read-only (`&`), so the surface never mutates the shooter (the actual TU /
/// ammo debit is the sim's `fire()` job, downstream of the emitted message).
pub type ShooterFireData<'a> = (&'a LifeState, &'a Tu, &'a TuMax, &'a Aiming, &'a Magazine);

/// Assemble a [`FireRequested`] for `shooter` against `target`, or [`None`] when the
/// shared `can_fire` guard fails (GTW-227's guard, folded verbatim for GTW-238).
///
/// The FIRE rung of [`crate::selection::left_click_act`]'s precedence chain:
///
/// 1. queries the shooter's [`FireActor`](gdtf_battle_sim::FireActor) components off
///    `shooters` (a `None` lookup — an unarmed/incomplete shooter — fails closed),
/// 2. runs the SHARED [`can_fire`](gdtf_battle_sim::can_fire) guard with the
///    [`SelectedFireMode`], the target cell / level, and the [`CombatTuning`],
/// 3. ONLY when it passes, returns
///    [`Some`]`(`[`FireRequested`]` { shooter, mode = *SelectedFireMode, target_cell,
///    target_level })` — which the unified system pushes as an
///    [`ActIntent::Fire`](crate::ActIntent::Fire).
///
/// Returns [`None`] (no fire) when: the shooter has no firing components; or `can_fire`
/// is `false` (a Downed/Dead shooter, an empty magazine, insufficient TU, or an
/// out-of-bounds target — AC3's `can_fire` rung). A read-only helper borrowing the
/// caller's `shooters` query (`bevy-traps.md` #7 — no `&mut World`).
#[must_use]
pub(crate) fn try_fire_request(
    shooter: Entity,
    target: CellLevel,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    shooters: &Query<ShooterFireData>,
) -> Option<FireRequested> {
    // The shooter must carry the firing components, else fail-closed (no fire).
    let Ok((life, tu, tu_max, aiming, magazine)) = shooters.get(shooter) else {
        return None;
    };
    let actor = FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
    };
    let (target_cell, target_level) = cell_and_level(target);

    // The SHARED guard set — the same `can_fire` the 222c button and the sim `fire()`
    // consult (resolution.md §9). LOS/fog is presenter policy, not a `can_fire` input.
    // `fire_mode` auto-derefs `SelectedFireMode` -> `FireModeSpec` to the `&FireModeSpec`
    // the guard expects.
    if !can_fire(&actor, fire_mode, target_cell, target_level, tuning) {
        return None;
    }

    Some(FireRequested::new(
        shooter,
        // `FireModeSpec` is `Copy` again (GTW-260) — copy the selected mode into the
        // owned `FireRequested` payload.
        **fire_mode,
        target_cell,
        target_level,
    ))
}

/// Splits a [`CellLevel`] into its ground-plane [`Cell`] and storey [`Level`] for the
/// [`FireRequested`] payload.
///
/// A [`CellLevel`] `Deref`s to an `IVec3` whose `x`/`y` are the cell and `z` is the
/// storey index; this reconstructs the typed [`Cell`] / [`Level`] (the same
/// decomposition the hover highlight's `level_index` uses). The storey `z` is always
/// `0..`[`MAX_LEVELS`](gdtf_battle_sim::MAX_LEVELS) (well within `u8`); the clamped
/// [`u8::try_from`] is the no-`unwrap` narrow — an (impossible) out-of-range `z`
/// saturates to [`u8::MAX`], which the shared `can_fire` `in_bounds` check then
/// rejects.
fn cell_and_level(target: CellLevel) -> (Cell, Level) {
    let cell = Cell::new(target.x, target.y);
    let level = Level::new(u8::try_from(target.z).unwrap_or(u8::MAX));
    (cell, level)
}
