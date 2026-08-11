use bevy::prelude::World;

pub(super) use super::super::{
    Actor, DownedTarget, can_execute, can_stabilize, execute_downed, execute_tu_cost,
    is_8_adjacent, stabilize_downed, stabilize_tu_cost,
};
pub(super) use crate::{
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position, Tu},
    metric::{Cell, CellLevel, Level},
    tuning::{CombatTuning, ExecuteTu, StabilizeTu},
};

pub(super) const GROUND: Level = Level::new(0);

// A pool well clear of either downed act's price.
pub(super) fn funded() -> Tu {
    Tu::new(100)
}

pub(super) fn one_short(cost: Tu) -> Tu {
    Tu::new((*cost).saturating_sub(1))
}

pub(super) fn pos(x: i32, y: i32, level: Level) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), level))
}

pub(super) fn actor(pos: Position, life: LifeState, faction: u8) -> Actor {
    Actor {
        pos,
        life,
        faction: Faction::new(faction),
    }
}

pub(super) fn target(
    pos: Position,
    life: LifeState,
    faction: u8,
    bleeding_out: bool,
) -> DownedTarget {
    DownedTarget {
        pos,
        life,
        faction: Faction::new(faction),
        bleeding_out: bleeding_out.then_some(BleedingOut),
    }
}

pub(super) fn stabilize_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, true);
    (a, t)
}

pub(super) fn execute_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, true);
    (a, t)
}

pub(super) fn run_stabilize(
    actor: &Actor,
    target: &DownedTarget,
    tu: Tu,
    tuning: &CombatTuning,
) -> (Option<StabilizeTu>, bool) {
    let mut world = World::new();
    let entity = world.spawn(BleedingOut).id();
    let cost = {
        let mut commands = world.commands();
        stabilize_downed(actor, target, entity, &mut commands, &tu, tuning)
    };
    world.flush();
    (cost, world.get::<BleedingOut>(entity).is_some())
}
