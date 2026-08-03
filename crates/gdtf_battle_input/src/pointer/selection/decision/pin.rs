//! Inspect-target pin decision for left-click.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::Faction;

use super::reads::LeftClickReads;
use crate::InspectTarget;

/// Whether to pin, unpin, or keep the inspect target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinOutcome {
    /// Pin this cell.
    Pin(gdtf_battle_sim::metric::CellLevel),
    /// Clear the pin.
    Unpin,
    /// Leave the pin unchanged.
    Keep,
}

/// Decide pin behavior from the hovered cell occupancy.
#[must_use]
pub fn decide_pin(
    reads: &LeftClickReads,
    inspect: &InspectTarget,
    factions: &Query<&Faction>,
) -> PinOutcome {
    let Some(cell) = inspect.hovered() else {
        return PinOutcome::Keep;
    };
    let player = **reads.player;
    let occupant = reads.occupancy.occupant(&cell);
    let occupant_faction = occupant.and_then(|e| factions.get(e).ok().copied());

    if *reads.occupancy.is_blocked(&cell) {
        return PinOutcome::Pin(cell);
    }
    if occupant_faction.is_some_and(|faction| faction != player) {
        return PinOutcome::Pin(cell);
    }
    if occupant_faction.is_some_and(|faction| faction == player) {
        return PinOutcome::Keep;
    }
    PinOutcome::Unpin
}

/// Apply a pin outcome to the inspect target.
pub fn apply_pin(outcome: PinOutcome, target: &mut ResMut<InspectTarget>) {
    match outcome {
        PinOutcome::Pin(cell) => {
            if target.pinned() != Some(cell) {
                target.set_pinned(cell);
            }
        }
        PinOutcome::Unpin => {
            if target.pinned().is_some() {
                target.clear_pin();
            }
        }
        PinOutcome::Keep => {}
    }
}
