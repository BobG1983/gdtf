//! The GTW-238 turn-to-face geometry decision shared by mouse right-click and gamepad East.

use bevy::prelude::*;
use gdtf_battle_sim::{
    acts::SetFacingRequested,
    prelude::{Direction, Position},
};

use crate::{InspectTarget, selection::SelectedShooter};

/// Resolves the turn-to-face decision (GTW-238) into an optional [`SetFacingRequested`] — the
/// SHARED turn decision (GTW-259) the mouse ([`right_click_turn_to_face`](crate::right_click_turn_to_face))
/// and the gamepad ([`gamepad_turn`](crate::gamepad::gamepad_turn), the East button) both use.
///
/// READ-ONLY: given the player-faction [`SelectedShooter`] (already faction-gated by the
/// caller), the [`InspectTarget`] (its LIVE hovered cell), and the actor [`Position`] query, it reads the actor's cell,
/// computes the target [`Direction`] toward the hovered cell via
/// [`Direction::from_cells`](gdtf_battle_sim::ganger::Direction::from_cells), and returns
/// [`Some`]`(`[`SetFacingRequested::new`]`)` — or [`None`] (no turn) when: there is no
/// selection; nothing is hovered; the actor has no [`Position`] (fail-closed via the query
/// lookup); or the hovered cell is the actor's OWN cell (`from_cells` returns [`None`]).
///
/// The per-45deg-step turn TU cost is the SIM's facing dispatch (GTW-235), NOT here. The
/// `selected` faction GATE (player-only) stays at each call site so this helper is purely the
/// geometric decision both surfaces share.
///
/// Param-only (`bevy-traps.md` #7): reads via the passed-in [`SelectedShooter`] +
/// [`InspectTarget`] (its LIVE hovered cell) + read-only `Query<&Position>`, no `&mut World`.
#[must_use]
pub fn decide_turn(
    selected: &SelectedShooter,
    hovered: &InspectTarget,
    positions: &Query<&Position>,
) -> Option<SetFacingRequested> {
    let actor = (**selected)?;
    // Nothing hovered -> no target cell. Faces the LIVE cursor cell, independent of any pin.
    let target = hovered.hovered()?;
    // The actor's grid cell (fail-closed if it has no Position component).
    let position = positions.get(actor).ok()?;
    // The canonical CellLevel::cell accessor (GTW-565; Position derefs to CellLevel).
    let actor_cell = position.cell();
    let hovered_cell = target.cell();
    // Hovering the actor's OWN cell yields no direction -> no-op (None).
    let facing = Direction::from_cells(actor_cell, hovered_cell)?;
    Some(SetFacingRequested::new(actor, facing))
}
