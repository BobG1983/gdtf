//! Mutates the targeting-hint Text node from the SHARED squad-visible read (GTW-11).

use bevy::prelude::*;
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::{CellVisibility, cell_squad_visible};
use gdtf_battle_sim::{Faction, FactionRelation, OccupancyGrid, PlayerFaction, SquadVisibility};

use crate::states::running::game::battlescape::targeting_hint::components::TargetingHintText;

/// The canon targeting-fog hint string (`docs/combat/visibility.md` §"UX edges" — VERBATIM,
/// exact em-dash + spacing). The hint reads EXACTLY this when the targeted cell is non-VISIBLE.
///
/// Framework plumbing — a literal `&str` handed to a [`Text`](bevy::prelude::Text) (NOT a domain
/// value; the hint STATE is the [`TargetingHintText`] marker, this is its rendered caption).
const HINT_TEXT: &str = "unseen — hold your fire";

/// Mutates the ONE targeting-hint Text node from the squad-visible verdict (GTW-11).
///
/// Reads the input crate's [`InspectTarget`] hovered cell and computes the cell's
/// [`CellVisibility`] via the SHARED
/// [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) predicate — the SAME read the
/// reticle recolour + the [`decide_left_click`](gdtf_battle_input::selection::decide_left_click)
/// fire-refusal consume, so the hint + the act can NEVER disagree (`docs/combat/visibility.md`
/// §"UX edges"). When the hovered cell is NOT squad-VISIBLE (UNSEEN *or* merely EXPLORED, both
/// alike — C5) the hint Text is set to the canon [`HINT_TEXT`] and shown
/// ([`Visibility::Visible`]); otherwise (a VISIBLE cell, or nothing hovered) the Text is cleared
/// and hidden ([`Visibility::Hidden`]). It MUTATES the existing node in place (never respawns) —
/// the sibling status-panel idiom.
///
/// The occupant's relation routes the verdict through `is_ganger_visible` (your own ganger is
/// always visible; an enemy iff its cell is VISIBLE) so the hint matches the reticle exactly; a
/// blocking-terrain cell carries no occupant (relation `None`). The squad fog / occupancy / player
/// faction are read as `Option<Res<…>>` so the system FAILS CLOSED (treats the cell as
/// non-VISIBLE) when a resource is absent, and never panics (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): `Res<InspectTarget>` + `Option<Res<…>>` reads + a read-only
/// `Query<&Faction>` + a `Query<(&mut Text, &mut Visibility), With<TargetingHintText>>` write; no
/// `&mut World`. Gated `run_if(resource_exists::<BattleInProgress>)` by the scene plugin.
pub(in crate::states::running::game::battlescape) fn update_targeting_hint(
    target: Res<InspectTarget>,
    grid: Option<Res<OccupancyGrid>>,
    squad: Option<Res<SquadVisibility>>,
    player: Option<Res<PlayerFaction>>,
    factions: Query<&Faction>,
    mut hints: Query<(&mut Text, &mut Visibility), With<TargetingHintText>>,
) {
    // The hovered cell's squad-visible verdict, or `SquadVisible` when nothing is hovered (no
    // cell -> no refusal -> no hint).
    let verdict = target
        .hovered()
        .map_or(CellVisibility::SquadVisible, |cell| {
            let relation = grid
                .as_ref()
                .and_then(|grid| grid.occupant(&cell))
                .map(|occupant| occupant_relation(occupant, &factions, player.as_deref()));
            cell_squad_visible(squad.as_deref(), &cell, relation)
        });
    let show = !verdict.is_squad_visible();

    for (mut text, mut visibility) in &mut hints {
        if show {
            // Set the canon string only when it differs, to avoid a needless re-layout.
            if text.as_str() != HINT_TEXT {
                HINT_TEXT.clone_into(&mut **text);
            }
            *visibility = Visibility::Visible;
        } else {
            if !text.is_empty() {
                text.clear();
            }
            *visibility = Visibility::Hidden;
        }
    }
}

/// The occupant's faction relation to the player squad — [`FactionRelation::OwnSquad`] when its
/// [`Faction`] equals the [`PlayerFaction`], else [`FactionRelation::Other`].
///
/// Fail-closed: an occupant with NO `Faction` component, or an absent `PlayerFaction`, is treated
/// as [`FactionRelation::Other`] (mirrors the input crate's reticle emitter so hint + reticle
/// resolve the SAME relation).
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
