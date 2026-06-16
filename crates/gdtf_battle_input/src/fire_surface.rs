//! The left-click FIRE surface (GTW-227 / GTW-48 S8 222b): the system that turns a
//! left-click on a target cell into a guarded fire intent.
//!
//! There is NO fire BUTTON — fire is left-click-on-the-target (the 222c action bar
//! carries stance / aim / mode / level only). On a left-mouse press, with a
//! [`SelectedShooter`] set and a [`HoveredCell`], this system assembles the shooter's
//! [`FireActor`](gdtf_battle_sim::FireActor) from its QUERIED components, runs the
//! SHARED [`can_fire`](gdtf_battle_sim::can_fire) guard set (the SAME validation the
//! 222c button and the sim `fire()` consult — resolution.md §9 "button and act share
//! one guard set"), and ONLY when it passes pushes an [`ActIntent::Fire`] carrying a
//! [`FireRequested`](gdtf_battle_sim::acts::FireRequested) `{ shooter =
//! *SelectedShooter, mode = *SelectedFireMode, target from HoveredCell }`. The ONE
//! [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits the carried
//! message. With no selection, no hovered cell, or a failing guard, NO intent is
//! written (AC4 / AC6).
//!
//! `can_fire` takes NO LOS input — LOS / fog is presenter policy, out of scope here
//! (resolution.md §"What's pure math vs sim"). Param-only (`bevy-traps.md` #7): all
//! reads via `Res` / `Query`, the intent push via `ResMut<PendingActIntent>`; no
//! `&mut World`.

use bevy::prelude::*;
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, FireActor, Level, LifeState, Magazine, Tu, TuMax, acts::FireRequested,
    can_fire, tuning::CombatTuning,
};

use crate::{ActIntent, HoveredCell, PendingActIntent, SelectedFireMode, SelectedShooter};

/// The shooter components [`fire_on_click`] queries to assemble a
/// [`FireActor`](gdtf_battle_sim::FireActor) for the shared `can_fire` guard.
///
/// The exact five reads `FireActor` borrows (`magazine.rs`): the shooter's
/// [`LifeState`], current [`Tu`], [`TuMax`], [`Aiming`] flag, and [`Magazine`]. A
/// `QueryData` tuple of borrowed sim components — framework plumbing, not a domain
/// scalar. Read-only (`&`), so the system never mutates the shooter (the actual TU /
/// ammo debit is the sim's `fire()` job, downstream of the emitted message).
type ShooterFireData<'a> = (&'a LifeState, &'a Tu, &'a TuMax, &'a Aiming, &'a Magazine);

/// Pushes an [`ActIntent::Fire`] on a left-click of a target cell, guarded by the
/// shared `can_fire` set (GTW-227 / 222b).
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Left)`, with a [`SelectedShooter`]
/// set and a [`HoveredCell`] target:
///
/// 1. queries the shooter's [`FireActor`](gdtf_battle_sim::FireActor) components,
/// 2. runs the SHARED [`can_fire`](gdtf_battle_sim::can_fire) guard with the
///    [`SelectedFireMode`], the target cell / level, and the [`CombatTuning`],
/// 3. ONLY when it passes, pushes
///    [`ActIntent::Fire`]`(`[`FireRequested`]`)` carrying `{ shooter, mode =
///    *SelectedFireMode, target_cell, target_level }` onto the shared
///    [`PendingActIntent`] queue (the ONE drain emits the message).
///
/// Writes NO intent when: the button is not just-pressed; there is no selection; the
/// selected entity has no firing components (fail-closed via the query lookup);
/// nothing is hovered; or `can_fire` is `false` (a Downed/Dead shooter, an empty
/// magazine, insufficient TU, or an out-of-bounds target — AC4). Param-only
/// (`bevy-traps.md` #7).
pub fn fire_on_click(
    mouse: Res<ButtonInput<MouseButton>>,
    selected: Res<SelectedShooter>,
    fire_mode: Res<SelectedFireMode>,
    hovered: Res<HoveredCell>,
    tuning: Res<CombatTuning>,
    shooters: Query<ShooterFireData>,
    mut pending: ResMut<PendingActIntent>,
) {
    // Only act on the press edge; a held button does not re-fire.
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // No selection -> no fire (AC6).
    let Some(shooter) = **selected else {
        return;
    };
    // Nothing hovered -> no target -> no fire.
    let Some(target) = **hovered else {
        return;
    };
    // The shooter must carry the firing components, else fail-closed (no fire).
    let Ok((life, tu, tu_max, aiming, magazine)) = shooters.get(shooter) else {
        return;
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
    // `&fire_mode` auto-derefs through `Res` -> `SelectedFireMode` -> `FireModeSpec`
    // to the `&FireModeSpec` the guard expects.
    if !can_fire(&actor, &fire_mode, target_cell, target_level, &tuning) {
        return;
    }

    pending.push(ActIntent::Fire(FireRequested::new(
        shooter,
        **fire_mode,
        target_cell,
        target_level,
    )));
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
