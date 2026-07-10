//! The FIRE decision (GTW-227 / GTW-238): the shared `can_fire` guard set folded into
//! a reusable helper the unified left-click surface ([`crate::selection::left_click_act`])
//! consults as the FIRST rung of its precedence chain.
//!
//! There is NO fire BUTTON — fire is left-click-on-an-ENEMY-target with a fire mode
//! selected (the 222c action bar carries stance / aim / mode / level only; an explicit
//! fire button is GTW-11). This module assembles the shooter's
//! [`FireActor`](gdtf_battle_sim::magazine::FireActor) from its QUERIED components and runs the
//! SHARED [`can_fire`](gdtf_battle_sim::magazine::can_fire) guard set (the SAME validation the
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
    acts::FireRequested,
    fire::MeleeQuery,
    ganger::{Aiming, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::{FireActor, Magazine, can_fire},
    prelude::{CellLevel, LifeState, Tu},
    tuning::CombatTuning,
    weapon::{Handedness, WieldedBy, Wields},
};

use crate::SelectedFireMode;

/// The shooter VITALS the FIRE branch queries off the GANGER to assemble a
/// [`FireActor`](gdtf_battle_sim::magazine::FireActor) for the shared `can_fire` guard.
///
/// The shooter vitals `FireActor` borrows from the ganger: its [`LifeState`], current
/// [`Tu`], [`TuMax`], and [`Aiming`] flag, plus its OPTIONAL [`InflictedInjuries`] ledger
/// (GTW-443 — folded into the [`FireActor`](gdtf_battle_sim::magazine::FireActor)'s
/// [`HandsAvailable`] hand count; an absent ledger = the uninjured two-hands default).
/// The [`Magazine`] + [`Handedness`] `FireActor` fields live on the related WEAPON entity
/// since GTW-323 slice 3 (ADR-0004) — read separately off `ganger → `[`Wields`]` → the
/// weapon entity` ([`WeaponMagazine`]), NOT off the ganger. A `QueryData` tuple of
/// borrowed sim components — framework plumbing, not a domain scalar. Read-only (`&`), so
/// the surface never mutates the shooter (the actual TU / ammo debit is the sim's
/// `fire()` job, downstream of the emitted message).
pub type ShooterFireData<'a> = (
    &'a LifeState,
    &'a Tu,
    &'a TuMax,
    &'a Aiming,
    Option<&'a InflictedInjuries>,
);

/// The wielded-weapon [`Magazine`] + [`Handedness`] read off a WEAPON entity
/// (`With<`[`WieldedBy`]`>`) — two [`FireActor`](gdtf_battle_sim::magazine::FireActor) fields,
/// resolved through the shooter's [`Wields`] relationship since GTW-323 slice 3 (ADR-0004;
/// [`Handedness`] added GTW-443). Read-only; a `QueryData` borrow filtered to weapon
/// entities so it never collides with the ganger-vitals [`ShooterFireData`] query.
pub type WeaponMagazine<'a> = (&'a Magazine, &'a Handedness);

/// Assemble a [`FireRequested`] for `shooter` against `target`, or [`None`] when the
/// shared `can_fire` guard fails (GTW-227's guard, folded verbatim for GTW-238).
///
/// The FIRE rung of [`crate::selection::left_click_act`]'s precedence chain:
///
/// 1. queries the shooter's [`FireActor`](gdtf_battle_sim::magazine::FireActor) components off
///    `shooters` (a `None` lookup — an unarmed/incomplete shooter — fails closed),
/// 2. runs the SHARED [`can_fire`](gdtf_battle_sim::magazine::can_fire) guard with the
///    [`SelectedFireMode`], the target cell / level, and the [`CombatTuning`],
/// 3. ONLY when it passes, returns
///    [`Some`]`(`[`FireRequested`]` { shooter, mode = *SelectedFireMode, target_cell,
///    target_level })` — which the unified system pushes as an
///    [`ActIntent::Fire`](crate::ActIntent::Fire).
///
/// Returns [`None`] (no fire) when: the shooter has no firing components / wields no
/// ranged weapon; or `can_fire` is `false` (a Downed/Dead shooter, an empty magazine,
/// insufficient TU, or an out-of-bounds target — AC3's `can_fire` rung). A read-only
/// helper borrowing the caller's `shooters` / `wields` / `weapons` / `melee` queries
/// (`bevy-traps.md` #7 — no `&mut World`).
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-505 C5: the ranged-weapon resolution adds the MeleeWeapon marker probe on top of \
              the GTW-323 slice-3 Wields + weapon-magazine queries, so the gun's magazine resolves \
              excluding the melee weapon the ganger also wields"
)]
#[must_use]
pub(crate) fn try_fire_request(
    shooter: Entity,
    target: CellLevel,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    shooters: &Query<ShooterFireData>,
    wields: &Query<&Wields>,
    weapons: &Query<WeaponMagazine, With<WieldedBy>>,
    melee: &MeleeQuery,
) -> Option<FireRequested> {
    // The shooter must carry the firing VITALS, else fail-closed (no fire).
    let Ok((life, tu, tu_max, aiming, injuries)) = shooters.get(shooter) else {
        return None;
    };
    // The magazine + handedness live on the wielded WEAPON entity (GTW-323 slice 3 /
    // GTW-443): resolve `ganger → Wields → the RANGED weapon entity → (Magazine, Handedness)`.
    // GTW-505 C5: a ganger wields BOTH a ranged AND a melee weapon, so resolve through
    // `Wields::ranged_weapon` (which EXCLUDES the `MeleeWeapon`-marked entity via the `melee`
    // probe) — NOT `Wields::weapon` (the FIRST related entity, which is spawn-order fragile).
    // This is the same marker-filtered resolution the sim's `fire()` / `can_fire` use, so the
    // input fire-guard reads the GUN's magazine regardless of relate order. An unarmed shooter
    // (no `Wields`, no ranged weapon, or no `Magazine` on it) fails closed (`?` -> `None`).
    let (magazine, handedness) = wields
        .get(shooter)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| weapons.get(weapon).ok())?;
    // GTW-443: fold the shooter's available hand count from its injury ledger (an absent
    // ledger = the uninjured two-hands default) so the shared can_fire hand-count gate
    // refuses a TwoHanded weapon when a hand-disabling injury has dropped it below two.
    let hands_available =
        injuries.map_or_else(HandsAvailable::default, InflictedInjuries::hands_available);
    let actor = FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness: *handedness,
        hands_available,
    };
    // The canonical CellLevel::split decompose (GTW-565) for the FireRequested
    // payload. The storey narrow rides the accessor's single clamp — an (impossible)
    // out-of-range `z` clamps into `0..=u8::MAX` instead of panicking; every real
    // key's storey is `0..MAX_LEVELS`, where the shared `can_fire` `in_bounds` check
    // remains the authoritative range gate.
    let (target_cell, target_level) = target.split();

    // The SHARED guard set — the same `can_fire` the 222c button and the sim `fire()`
    // consult (resolution.md §9). LOS/fog is presenter policy, not a `can_fire` input.
    // `fire_mode` auto-derefs `SelectedFireMode` -> `FireModeSpec` to the `&FireModeSpec`
    // the guard expects.
    if !*can_fire(&actor, fire_mode, target_cell, target_level, tuning) {
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
