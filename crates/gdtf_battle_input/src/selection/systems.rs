//! The mouse click control systems (GTW-238): [`left_click_act`] (the unified FIRE → SELECT →
//! MOVE → CLEAR decision) and [`right_click_turn_to_face`], both gated to the player's faction.

use bevy::prelude::*;
use gdtf_battle_sim::{Faction, PlayerFaction, Position};

use crate::{
    ActIntent, InspectTarget, PendingActIntent,
    fire_surface::ShooterFireData,
    selection::{
        decision::{
            LeftClickReads, TurnReads, apply_left_click, apply_pin, decide_left_click, decide_pin,
            decide_turn,
        },
        resources::SelectedShooter,
    },
};

/// Resolves ONE Left-press edge through the FIRE → SELECT → MOVE → CLEAR precedence chain
/// (GTW-238), gated to the player's own faction.
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Left)`, [`decide_left_click`] resolves the
/// SHARED outcome (the cell resolved last update) and [`apply_left_click`] commits it. The
/// decision is the SAME one [`gamepad_click_act`](crate::gamepad::gamepad_click_act) (the South
/// button) uses — ONE precedence implementation, two press surfaces (GTW-259). The precedence:
///
/// 1. **FIRE** — a fire mode is selected, the hovered cell holds an ENEMY occupant
///    (a [`Faction`] `!=` [`PlayerFaction`]), the current selection is a player-faction ganger,
///    and the shared [`can_fire`](gdtf_battle_sim::can_fire) guard passes → push
///    [`ActIntent::Fire`]; nothing else changes this edge.
/// 2. **SELECT** — the hovered cell holds one of YOUR gangers
///    ([`Faction`] `==` [`PlayerFaction`]) → set [`SelectedShooter::new`]; no act emitted.
/// 3. **MOVE** — there is a player-faction selection AND the hovered cell is empty, in-bounds,
///    and unblocked → push [`ActIntent::Move`] to the hovered destination.
/// 4. **CLEAR** — none of the above → [`SelectedShooter::cleared`] (still fires for an empty /
///    blocked cell with a non-player or stale selection).
///
/// Between MOVE (clause 3) and CLEAR (clause 4) sits the GTW-287 NO-OP rung: an enemy-occupied
/// click that does not FIRE does NOTHING — it NEVER clears the player's selection (enemies are
/// inspected via the GTW-274 hover panel) — so no transient `None`, no "No ganger selected"
/// flash.
///
/// FALL-THROUGH (the user-confirmed precedence): a fire mode over an EMPTY / your-own /
/// non-enemy cell FAILS clause 1 and falls through to clause 3 (MOVE). The branches are MUTUALLY
/// EXCLUSIVE: a FIRE edge emits no [`MoveRequested`](gdtf_battle_sim::acts::MoveRequested) and
/// does not touch [`SelectedShooter`]; a SELECT edge emits no act message; an enemy-click NO-OP
/// (GTW-287) touches nothing.
///
/// GTW-300 — the SAME edge also resolves the PARALLEL inspect-panel pin
/// ([`decide_pin`] / [`apply_pin`]): clicking cover or an enemy PINS the panel on that cell,
/// clicking an empty tile UNPINS, and a SELECT / FIRE / no-hover click leaves the pin untouched.
/// The pin is ORTHOGONAL to the FIRE / SELECT / MOVE / CLEAR effect (a different resource,
/// [`InspectTarget`]), so both effects compose on one click with no double-dispatch.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + read-only
/// `Query<&Faction>` + `Query<ShooterFireData>`, the [`ResMut<SelectedShooter>`] /
/// [`ResMut<PendingActIntent>`] / [`ResMut<InspectTarget>`] writes — no `&mut World`. Runs
/// `.before(pick_hovered_cell)` (`bevy-traps.md` #3) so it reads the cell resolved last update,
/// and `.before(dispatch_act_intents)` so the drain sees this update's pushes.
pub fn left_click_act(
    reads: LeftClickReads,
    factions: Query<&Faction>,
    shooters: Query<ShooterFireData>,
    mut selected: ResMut<SelectedShooter>,
    mut pending: ResMut<PendingActIntent>,
    mut inspect: ResMut<InspectTarget>,
) {
    // Only act on the press edge; a held button does not re-resolve.
    if !reads.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Decide both effects from the immutable inspect target FIRST, then commit them (the pin
    // write borrows `inspect` mutably, so the read-only decisions must finish before it).
    let outcome = decide_left_click(&reads, &inspect, &factions, &shooters, &selected);
    let pin = decide_pin(&reads, &inspect, &factions);
    apply_left_click(outcome, &mut selected, &mut pending);
    // GTW-300 — the parallel pin effect (orthogonal to the act/selection effect above).
    apply_pin(pin, &mut inspect);
}

/// Turns the player-faction [`SelectedShooter`] to face the LIVE hovered cell
/// ([`InspectTarget::hovered`](crate::InspectTarget::hovered)) on a Right press (GTW-238),
/// pushing an [`ActIntent::Turn`].
///
/// On a `ButtonInput<MouseButton>` `just_pressed(Right)` with a player-faction
/// [`SelectedShooter`], it applies the SHARED [`decide_turn`] decision (the SAME one
/// [`gamepad_turn`](crate::gamepad::gamepad_turn) uses, GTW-259) and pushes the resulting
/// [`ActIntent::Turn`]`(`[`SetFacingRequested`](gdtf_battle_sim::acts::SetFacingRequested)`)`.
/// The per-45deg-step turn TU cost is the SIM's facing dispatch (GTW-235), NOT here.
///
/// Writes NO intent when: the button is not just-pressed; there is no selection; the selection
/// is NOT a player-faction ganger (the faction gate stays here, AC6); or [`decide_turn`] returns
/// [`None`].
///
/// Param-only (`bevy-traps.md` #7): all reads via `Res` / `Query`, the intent push via
/// [`ResMut<PendingActIntent>`]; no `&mut World`. Runs `.before(pick_hovered_cell)` (the cell
/// resolved last update) and `.before(dispatch_act_intents)` (the drain).
pub fn right_click_turn_to_face(
    mouse: Res<ButtonInput<MouseButton>>,
    reads: TurnReads,
    factions: Query<&Faction>,
    positions: Query<&Position>,
    mut pending: ResMut<PendingActIntent>,
) {
    // Only act on the press edge; a held button does not re-turn.
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    // Gating: only a PLAYER-faction selection turns (a forced enemy emits nothing, AC6).
    if !selection_is_player(*reads.selected, &factions, *reads.player) {
        return;
    }
    if let Some(request) = decide_turn(&reads.selected, &reads.hovered, &positions) {
        pending.push(ActIntent::Turn(request));
    }
}

/// Whether the current [`SelectedShooter`] is one of the player's own gangers (its [`Faction`]
/// equals [`PlayerFaction`]).
///
/// The player-faction gate the turn surfaces apply before [`decide_turn`] (a forced enemy
/// selection emits nothing, AC6). Read-only — looks the selection up in the `Query<&Faction>`
/// and compares to the player faction; `false` when nothing is selected or the selection carries
/// no [`Faction`].
fn selection_is_player(
    selected: SelectedShooter,
    factions: &Query<&Faction>,
    player: PlayerFaction,
) -> bool {
    (*selected)
        .and_then(|actor| factions.get(actor).ok().copied())
        .is_some_and(|faction| faction == *player)
}
