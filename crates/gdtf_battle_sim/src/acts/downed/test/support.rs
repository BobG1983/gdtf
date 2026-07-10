//! Shared fixtures for the relocated `downed_acts` tests — the cell/level position
//! builder, the [`Actor`]/[`DownedTarget`] bundle constructors, and the canonical
//! passing setups. Re-exported `pub(super)` so each per-AC sibling reuses them.

use bevy::prelude::World;

pub(super) use super::super::{
    Actor, DownedTarget, can_execute, can_stabilize, execute_downed, is_8_adjacent,
    stabilize_downed,
};
pub(super) use crate::{
    effects::bleed::BleedingOut,
    ganger::{Faction, LifeState, Position},
    metric::{Cell, CellLevel, Level},
    tuning::{CombatTuning, ExecuteTu, StabilizeTu},
};

/// The ground storey (level 0) — the shared storey the same-level fixtures sit
/// on (so the Moore-8 reach is exercised on one storey, with a deliberate
/// off-storey case for the cross-level rejection).
pub(super) const GROUND: Level = Level::new(0);

/// Build a [`Position`] from cell `(x, y)` on a storey — keeps the test bodies
/// terse and pixel-free (all cubic-voxel cell/level coordinates).
pub(super) fn pos(x: i32, y: i32, level: Level) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), level))
}

/// Build an [`Actor`] bundle from its three gating reads.
pub(super) fn actor(pos: Position, life: LifeState, faction: u8) -> Actor {
    Actor {
        pos,
        life,
        faction: Faction::new(faction),
    }
}

/// Build a [`DownedTarget`] bundle; `bleeding_out` sets whether the target carries the
/// §9 [`BleedingOut`] condition (`true` = clock running / stabilizable, `false` = already
/// stabilized).
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

/// The canonical passing stabilize setup: an Alive ally one cell from a Downed,
/// bleeding-out same-faction target.
pub(super) fn stabilize_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, true);
    (a, t)
}

/// The canonical passing execute setup: an Alive enemy one cell from a Downed
/// opposing-faction target.
pub(super) fn execute_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, true);
    (a, t)
}

/// Run [`stabilize_downed`] over a fresh `World` whose target entity starts carrying the
/// [`BleedingOut`] condition, exercising the real Commands removal path; returns the
/// verb's [`Option<StabilizeTu>`] verdict and whether the marker SURVIVED (still present
/// after the deferred removal flushes). A bare-`World` in a unit test body — the
/// `bevy-traps.md` #7 headless carve-out.
pub(super) fn run_stabilize(
    actor: &Actor,
    target: &DownedTarget,
    tuning: &CombatTuning,
) -> (Option<StabilizeTu>, bool) {
    let mut world = World::new();
    let entity = world.spawn(BleedingOut).id();
    let cost = {
        let mut commands = world.commands();
        stabilize_downed(actor, target, entity, &mut commands, tuning)
    };
    world.flush();
    (cost, world.get::<BleedingOut>(entity).is_some())
}
