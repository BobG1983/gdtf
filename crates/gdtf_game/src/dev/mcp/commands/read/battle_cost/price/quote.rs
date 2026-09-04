//! The answer one priced act carries, and which act each call is routed to.

use bevy::prelude::*;
use gdtf_battle_sim::{ganger::Tu, posture::set_aiming_tu_cost, tu::can_spend_tu};

use super::{gear, posture, reach, walk};
use crate::dev::mcp::{
    commands::{
        act::support::a_ganger,
        read::battle_cost::reads::{ActorRowItem, CostRows, LoadedWorld},
    },
    wire::{
        cost::{CostActNet, CostLegalNet, CostRefusalNet},
        token::GangerToken,
    },
};

/// What one act would cost, and why the sim would refuse it.
pub(in crate::dev::mcp::commands::read::battle_cost) struct Quote {
    cost:    Option<Tu>,
    refusal: Option<CostRefusalNet>,
}

impl Quote {
    /// A priced act, refused or not.
    pub(super) const fn quoted(cost: Tu, refusal: Option<CostRefusalNet>) -> Self {
        Self {
            cost: Some(cost),
            refusal,
        }
    }

    /// A refusal that never got as far as a price.
    pub(super) const fn refused(refusal: CostRefusalNet) -> Self {
        Self {
            cost:    None,
            refusal: Some(refusal),
        }
    }

    /// The TU the sim quoted, absent when the act did not resolve.
    pub(in crate::dev::mcp::commands::read::battle_cost) const fn cost(&self) -> Option<Tu> {
        self.cost
    }

    /// Why the act would be refused, absent when it would be allowed.
    pub(in crate::dev::mcp::commands::read::battle_cost) const fn refusal(
        &self,
    ) -> Option<CostRefusalNet> {
        self.refusal
    }
}

/// Price one act for the ganger a token names.
pub(in crate::dev::mcp::commands::read::battle_cost) fn quote(
    world: &LoadedWorld,
    rows: &CostRows,
    actor: GangerToken,
    act: CostActNet,
) -> Quote {
    let Some(entity) = a_ganger(&rows.tokens, actor) else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    let Ok(row) = rows.actors.get(entity) else {
        return Quote::refused(CostRefusalNet::NoSuchGanger);
    };
    if *row.faction != world.player {
        return Quote::refused(CostRefusalNet::NotYourGanger);
    }
    priced(world, rows, entity, &row, act)
}

fn priced(
    world: &LoadedWorld,
    rows: &CostRows,
    actor: Entity,
    row: &ActorRowItem<'_, '_>,
    act: CostActNet,
) -> Quote {
    let tuning = world.tuning;
    match act {
        CostActNet::Move { dest } => walk::move_quote(world, rows, actor, row, dest),
        CostActNet::Fire { target, mode } => {
            gear::fire_quote(rows, tuning, actor, row, target, mode)
        }
        CostActNet::Reload => gear::reload_quote(rows, actor, row),
        CostActNet::SetStance { stance } => posture::stance_quote(tuning, row, stance),
        CostActNet::SetFacing { facing } => posture::facing_quote(tuning, row, facing),
        CostActNet::SetAiming { .. } => {
            afforded(*row.tu, set_aiming_tu_cost(), CostLegalNet::new(true))
        }
        CostActNet::Shove { target } => reach::shove_quote(rows, tuning, row, target),
        CostActNet::OpenDoor { target } => reach::open_door_quote(rows, tuning, row, target),
        CostActNet::EnterEmplacement { target } => reach::enter_quote(rows, tuning, row, target),
        CostActNet::ExitEmplacement { target } => {
            reach::exit_quote(world, rows, actor, row, target)
        }
        CostActNet::ThrowGrenade { .. } => gear::throw_quote(rows, tuning, actor, row),
        CostActNet::Melee { target } => gear::melee_quote(world, rows, actor, row, target),
    }
}

/// A priced act is refused when the pool cannot cover it, then when the sim says no.
pub(super) fn afforded(tu: Tu, cost: Tu, allowed: CostLegalNet) -> Quote {
    if !*can_spend_tu(&tu, cost) {
        return Quote::quoted(cost, Some(CostRefusalNet::CannotAfford));
    }
    Quote::quoted(cost, (!*allowed).then_some(CostRefusalNet::ActNotAllowed))
}
