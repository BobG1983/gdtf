//! What the acts priced against something next to the actor cost.

use bevy::prelude::*;
use gdtf_battle_sim::{
    acts::{
        ShoveActor, ShoveTarget, can_enter_emplacement, can_exit_emplacement, can_open_door,
        can_shove, enter_emplacement_tu_cost, exit_emplacement_tu_cost, open_door_tu_cost,
        shove_tu_cost,
    },
    ganger::Position,
    tuning::CombatTuning,
};

use super::quote::{Quote, afforded};
use crate::dev::net_qa::{
    commands::{
        act::support::a_ganger,
        read::battle_cost::reads::{ActorRowItem, CostRows},
    },
    wire::{
        cost::{CostLegalNet, CostRefusalNet},
        token::{DoorToken, EmplacementToken, GangerToken},
    },
};

/// Price shoving the ganger `target` names.
pub(super) fn shove_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    row: &ActorRowItem<'_, '_>,
    target: GangerToken,
) -> Quote {
    let Some((position, faction, life)) =
        a_ganger(&rows.tokens, target).and_then(|entity| rows.targets.get(entity).ok())
    else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let cost = shove_tu_cost(tuning);
    let allowed = can_shove(
        &ShoveActor {
            position: *row.position,
            faction:  *row.faction,
        },
        &ShoveTarget {
            position: *position,
            faction:  *faction,
            life:     *life,
        },
    );
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price opening the door `target` names.
pub(super) fn open_door_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    row: &ActorRowItem<'_, '_>,
    target: DoorToken,
) -> Quote {
    let Some((open_state, cell)) =
        Entity::try_from_bits(*target).and_then(|door| rows.doors.get(door).ok())
    else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let cost = open_door_tu_cost(tuning);
    let allowed = can_open_door(
        *row.position,
        Position::new(**cell),
        *open_state,
        row.tu,
        tuning,
    );
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price taking the emplacement `target` names.
pub(super) fn enter_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    row: &ActorRowItem<'_, '_>,
    target: EmplacementToken,
) -> Quote {
    let Some((state, cell, _occupant)) =
        Entity::try_from_bits(*target).and_then(|seat| rows.emplacements.get(seat).ok())
    else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let cost = enter_emplacement_tu_cost(tuning);
    let allowed =
        can_enter_emplacement(*row.position, Position::new(**cell), state, row.tu, tuning);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price leaving the emplacement `target` names.
pub(super) fn exit_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
    target: EmplacementToken,
) -> Quote {
    let Some((state, _cell, occupant)) =
        Entity::try_from_bits(*target).and_then(|seat| rows.emplacements.get(seat).ok())
    else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let cost = exit_emplacement_tu_cost(tuning);
    let allowed =
        occupant.is_some_and(|held| *can_exit_emplacement(actor, state, held, row.tu, tuning));
    afforded(*row.tu, cost, CostLegalNet::new(allowed))
}
