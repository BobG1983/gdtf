//! Left-click outcome decision for select, fire, and move.

use bevy::prelude::*;
use gdtf_battle_presenter::cell_squad_visible;
use gdtf_battle_sim::{
    acts::{FireRequested, MoveRequested},
    fire::MeleeQuery,
    ganger::LifeState,
    prelude::{CellLevel, Faction},
    visibility::FactionRelation,
    weapon::{WieldedBy, Wields},
};

use super::reads::LeftClickReads;
use crate::{
    InspectTarget,
    fire_surface::{ShooterFireData, WeaponMagazine, try_fire_request},
    selection::{path_preview::PathPreviewTarget, resources::SelectedShooter},
};

/// Result of a left-click on the battlescape.
#[derive(Debug, Clone, PartialEq)]
pub enum LeftClickOutcome {
    /// Fire at the hovered cell or enemy.
    Fire(FireRequested),
    /// Select a player ganger.
    Select(Entity),
    /// Pin a move path preview target.
    SetMoveTarget(CellLevel),
    /// Confirm move to the already-pinned target.
    Move(MoveRequested),
    /// No action.
    NoOp,
    /// Clear selection and move target.
    Clear,
}

#[expect(
    clippy::too_many_arguments,
    reason = "GTW-356: the two-click move state machine reads the current PathPreviewTarget, which \
              must be passed as a separate `&` (not in LeftClickReads) so the caller holds the \
              ResMut for the target write without a B0002 Res+ResMut alias — the InspectTarget \
              precedent; on top of the GTW-323 slice-3 Wields + weapon-magazine queries and the \
              GTW-729 LifeState query gating the SELECT clause"
)]
/// Decide what a left-click should do given hover, selection, and fire mode.
#[must_use]
pub fn decide_left_click(
    reads: &LeftClickReads,
    inspect: &InspectTarget,
    move_target: &PathPreviewTarget,
    factions: &Query<&Faction>,
    lifes: &Query<&LifeState>,
    shooters: &Query<ShooterFireData>,
    wields: &Query<&Wields>,
    weapons: &Query<WeaponMagazine, With<WieldedBy>>,
    melee: &MeleeQuery,
    selected: &SelectedShooter,
) -> LeftClickOutcome {
    let Some(target) = inspect.hovered() else {
        return LeftClickOutcome::NoOp;
    };
    let player = **reads.player;
    let occupant = reads.occupancy.occupant(&target);
    let occupant_faction = occupant.and_then(|e| factions.get(e).ok().copied());
    let selection_is_player = (**selected)
        .and_then(|e| factions.get(e).ok().copied())
        .is_some_and(|f| f == player);

    if let Some(enemy_faction) = occupant_faction
        && selection_is_player
        && enemy_faction != player
        && !cell_squad_visible(
            reads.squad_visibility.as_deref(),
            &target,
            Some(FactionRelation::Other),
        )
        .is_squad_visible()
    {
        return LeftClickOutcome::NoOp;
    }

    if let (Some(shooter), Some(enemy_faction)) = (**selected, occupant_faction)
        && selection_is_player
        && enemy_faction != player
        && let Some(request) = try_fire_request(
            shooter,
            target,
            &reads.fire_mode,
            &reads.tuning,
            shooters,
            wields,
            weapons,
            melee,
        )
    {
        return LeftClickOutcome::Fire(request);
    }

    if let Some(shooter) = **selected
        && selection_is_player
        && occupant.is_none()
        && *reads.occupancy.is_blocked(&target)
        && cell_squad_visible(reads.squad_visibility.as_deref(), &target, None).is_squad_visible()
        && let Some(request) = try_fire_request(
            shooter,
            target,
            &reads.fire_mode,
            &reads.tuning,
            shooters,
            wields,
            weapons,
            melee,
        )
    {
        return LeftClickOutcome::Fire(request);
    }

    if let (Some(entity), Some(faction)) = (occupant, occupant_faction)
        && faction == player
        && selectable_by_life(lifes, entity)
    {
        return LeftClickOutcome::Select(entity);
    }

    if occupant_faction.is_some_and(|faction| faction == player) {
        return LeftClickOutcome::NoOp;
    }

    if let Some(actor) = **selected
        && selection_is_player
        && occupant.is_none()
        && !*reads.occupancy.is_blocked(&target)
    {
        if reads.links.links_from(&target).next().is_some() {
            return LeftClickOutcome::NoOp;
        }
        return if **move_target == Some(target) {
            LeftClickOutcome::Move(MoveRequested::new(actor, target))
        } else {
            LeftClickOutcome::SetMoveTarget(target)
        };
    }

    if occupant_faction.is_some_and(|faction| faction != player) {
        return LeftClickOutcome::NoOp;
    }

    LeftClickOutcome::Clear
}

fn selectable_by_life(lifes: &Query<&LifeState>, entity: Entity) -> bool {
    match lifes.get(entity) {
        Ok(life) => *life.is_active(),
        Err(_) => true,
    }
}
