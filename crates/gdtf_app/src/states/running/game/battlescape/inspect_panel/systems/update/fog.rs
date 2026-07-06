//! The GTW-378 squad-visibility gate for hovered occupants: whether a hovered ganger
//! may feed the inspect panel (fail-closed, the shared fog read). Split out of the
//! monolithic `update.rs` (GTW-583); the repaint rationale lives on the parent
//! `update` module.

use bevy::prelude::*;
use gdtf_battle_presenter::cell_squad_visible;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{CellLevel, Faction},
    visibility::FactionRelation,
};

use super::params::InspectReads;

/// Whether a hovered `occupant` at `cell` is currently squad-VISIBLE (GTW-378) — the
/// targeting-fog gate that decides whether the occupant may feed the inspect panel.
///
/// Routes the occupant's [`Faction`] relation to the player squad through the SHARED
/// [`cell_squad_visible`] predicate (the SAME fail-closed read the reticle + the fire-refusal
/// use): an OWN-squad occupant is trivially visible (you always see your own); an ENEMY is
/// visible iff its cell is currently squad-VISIBLE — the SAME read that shows / hides its
/// sprite, so a fog-hidden enemy is treated as not-there for the panel (no info-leak).
/// FAIL-CLOSED: an absent fog (`reads.squad` is `None`) or an absent / unknown faction yields
/// NOT-visible, never populating the panel for a cell the squad's truth cannot vouch for.
pub(super) fn occupant_squad_visible(
    cell: CellLevel,
    occupant: Entity,
    factions: &Query<&Faction>,
    reads: &InspectReads,
) -> bool {
    let relation = occupant_relation(occupant, factions, reads.player.as_deref());
    cell_squad_visible(reads.squad.as_deref(), &cell, Some(relation)).is_squad_visible()
}

/// The occupant's [`FactionRelation`] to the player squad — [`FactionRelation::OwnSquad`] when
/// its [`Faction`] equals the [`PlayerFaction`], else [`FactionRelation::Other`] (GTW-378).
///
/// Fail-closed: an occupant with NO `Faction` component, or an absent `PlayerFaction`, is
/// treated as [`FactionRelation::Other`] (the enemy / not-yours case), so the fog gate never
/// claims a cell is yours — mirroring `gdtf_battle_input`'s `occupant_relation`.
fn occupant_relation(
    occupant: Entity,
    factions: &Query<&Faction>,
    player: Option<&PlayerFaction>,
) -> FactionRelation {
    let occupant_faction = factions.get(occupant).ok().copied();
    match (occupant_faction, player) {
        (Some(faction), Some(player)) if faction == **player => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}
