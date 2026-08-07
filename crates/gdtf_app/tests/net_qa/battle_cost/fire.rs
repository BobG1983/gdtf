//! Which fire mode a `battle.cost` call is priced against: the one it named, and no other.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    misc::ModeKindNet,
    vitals::TuNet,
};
use gdtf_battle_sim::weapon::{FireModeSpec, ModeKind};

use super::support::{CostBody, a_gun_with_modes, cost_body, cost_calls, fire_cost, settle};
use crate::{
    battle_reads::cell_of, command_exchange::exchange_in_battle, socket_support::TestResult,
};

/// Every fire mode kind a call can name.
const EVERY_KIND: [ModeKind; 3] = [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

/// A mode kind a call names, and the spec the gun offers for it, if it offers one at all.
#[derive(Debug, Clone, Copy)]
struct AskedMode {
    kind:    ModeKind,
    offered: Option<FireModeSpec>,
}

/// The shooter a case asks about, and every mode kind it asks for.
struct AskedGun {
    actor: Entity,
    modes: Vec<AskedMode>,
}

/// Every mode kind a call can name, paired with the spec the gun offers for it.
fn asked_modes(modes: &[FireModeSpec]) -> Vec<AskedMode> {
    EVERY_KIND
        .into_iter()
        .map(|kind| AskedMode {
            kind,
            offered: modes.iter().find(|spec| spec.kind == kind).copied(),
        })
        .collect()
}

/// One shot at `at` per mode kind, in the order the kinds are listed.
fn fire_at(at: CellLevelNet, asked: &[AskedMode]) -> Vec<CostActNet> {
    asked
        .iter()
        .map(|mode| CostActNet::Fire {
            target: at,
            mode:   ModeKindNet::from_sim(mode.kind),
        })
        .collect()
}

#[test]
fn a_fire_quote_prices_the_mode_the_call_named() -> TestResult {
    let mut planned: Option<AskedGun> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some((actor, modes)) = a_gun_with_modes(app) else {
            return Vec::new();
        };
        let Some(at) = cell_of(app, actor) else {
            return Vec::new();
        };
        let modes = asked_modes(&modes);
        let calls = cost_calls(actor, &fire_at(at, &modes));
        planned = Some(AskedGun { actor, modes });
        calls
    })?;
    let Some(gun) = planned else {
        return Err("the battle fixture must field a player ganger holding a ranged weapon".into());
    };
    assert_eq!(
        replies.len(),
        gun.modes.len(),
        "every mode asked about must come back with its own reply",
    );
    for (asked, reply) in gun.modes.iter().zip(replies) {
        let body = cost_body(reply)?;
        check_fire_quote(&app, gun.actor, *asked, &body)?;
    }
    Ok(())
}

/// A mode the gun offers is priced by its own spec; a mode it does not offer is refused unpriced.
fn check_fire_quote(app: &App, actor: Entity, asked: AskedMode, body: &CostBody) -> TestResult {
    let kind = asked.kind;
    let Some(spec) = asked.offered else {
        assert_eq!(
            body.refusal,
            Some(CostRefusalNet::ActNotAllowed),
            "a gun that has no {kind:?} mode cannot be quoted one: {body:?}",
        );
        assert_eq!(
            body.cost, None,
            "a mode the gun does not offer carries no price: {body:?}",
        );
        return Ok(());
    };
    let Some(expected) = fire_cost(app, actor, &spec) else {
        return Err("the same world must still price the mode the reply quoted".into());
    };
    assert_eq!(
        body.cost,
        Some(TuNet::new(*expected)),
        "a fire quote must be what mode_tu_cost charges for {kind:?}, not for another mode: \
         {body:?}",
    );
    Ok(())
}
