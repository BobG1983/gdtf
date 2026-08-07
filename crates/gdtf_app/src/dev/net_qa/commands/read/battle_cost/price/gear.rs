//! What the acts priced off the actor's own weapons cost: fire, reload, throw, melee.

use bevy::prelude::*;
use gdtf_battle_input::ranged_weapon_of;
use gdtf_battle_sim::{
    acts::{
        MeleeAttacker, MeleeReach, can_melee, can_reload, can_throw_grenade, melee_tu_cost,
        reload_tu_cost, throw_grenade_tu_cost,
    },
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::{FireActor, can_fire, mode_tu_cost},
    tuning::CombatTuning,
};

use super::quote::{Quote, afforded};
use crate::dev::net_qa::{
    commands::{
        act::support::a_ganger,
        read::battle_cost::reads::{ActorRowItem, CostRows},
    },
    wire::{
        act_payload::MeleeTargetNet,
        cell::CellLevelNet,
        cost::{CostLegalNet, CostRefusalNet},
        misc::ModeKindNet,
    },
};

/// Price one shot at `target` in `mode`, with the gun the actor is holding.
pub(super) fn fire_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
    target: CellLevelNet,
    mode: ModeKindNet,
) -> Quote {
    let Some(gun) = ranged_weapon_of(actor, &rows.wields, &rows.melee)
        .and_then(|weapon| rows.guns.get(weapon).ok())
    else {
        return Quote::refused(CostRefusalNet::ActNotAllowed);
    };
    let Some(spec) = gun
        .modes
        .iter()
        .find(|spec| spec.kind == mode.to_sim())
        .copied()
    else {
        return Quote::refused(CostRefusalNet::ActNotAllowed);
    };
    let cost = mode_tu_cost(&spec, row.tu_max, row.aiming, tuning);
    let at = target.to_sim();
    let shooter = FireActor {
        life:            row.life,
        tu:              row.tu,
        tu_max:          row.tu_max,
        aiming:          row.aiming,
        magazine:        gun.magazine,
        handedness:      *gun.handedness,
        hands_available: row
            .injuries
            .map_or_else(HandsAvailable::default, InflictedInjuries::hands_available),
    };
    let allowed = can_fire(&shooter, &spec, at.cell(), at.level(), tuning);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price refilling the magazine of the gun the actor is holding.
pub(super) fn reload_quote(rows: &CostRows, actor: Entity, row: &ActorRowItem<'_, '_>) -> Quote {
    let Some(gun) = ranged_weapon_of(actor, &rows.wields, &rows.melee)
        .and_then(|weapon| rows.guns.get(weapon).ok())
    else {
        return Quote::refused(CostRefusalNet::ActNotAllowed);
    };
    let cost = reload_tu_cost(gun.magazine);
    let allowed = can_reload(*row.life, row.tu, gun.magazine);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price one throw of the arc weapon the actor is holding.
pub(super) fn throw_quote(
    rows: &CostRows,
    tuning: &CombatTuning,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
) -> Quote {
    let Some(gun) = ranged_weapon_of(actor, &rows.wields, &rows.melee)
        .and_then(|weapon| rows.guns.get(weapon).ok())
    else {
        return Quote::refused(CostRefusalNet::ActNotAllowed);
    };
    let cost = throw_grenade_tu_cost(tuning);
    let allowed = can_throw_grenade(*gun.trajectory, gun.magazine, row.tu, tuning);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price one strike with the melee weapon the actor is holding.
pub(super) fn melee_quote(
    rows: &CostRows,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
    target: MeleeTargetNet,
) -> Quote {
    let Some(fight) = rows
        .wields
        .get(actor)
        .ok()
        .and_then(|held| held.melee_weapon(|entity| rows.melee.get(entity).is_ok()))
        .and_then(|weapon| rows.fights.get(weapon).ok())
    else {
        return Quote::refused(CostRefusalNet::ActNotAllowed);
    };
    let Some(reach) = melee_reach(rows, target) else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let cost = melee_tu_cost(fight);
    let allowed = can_melee(MeleeAttacker::new(*row.position, *row.faction), reach);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

fn melee_reach(rows: &CostRows, target: MeleeTargetNet) -> Option<MeleeReach> {
    match target {
        MeleeTargetNet::Ganger(token) => {
            let entity = a_ganger(&rows.tokens, token)?;
            let (position, faction, life) = rows.targets.get(entity).ok()?;
            Some(MeleeReach::ganger(*position, *faction, *life))
        }
        MeleeTargetNet::Structure(at) => Some(MeleeReach::structure(at.to_sim())),
    }
}
