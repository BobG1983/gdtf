//! Shared fixtures for the relocated `downed_acts` tests — the cell/level position
//! builder, the [`Actor`]/[`DownedTarget`] bundle constructors, and the canonical
//! passing setups. Re-exported `pub(super)` so each per-AC sibling reuses them.

pub(super) use super::super::{
    Actor, DownedTarget, can_execute, can_stabilize, execute_downed, is_8_adjacent,
    stabilize_downed,
};
pub(super) use crate::{
    ganger::{Faction, LifeState, Position, Stabilized},
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

/// Build a [`DownedTarget`] bundle; `stabilized` is the optional E3.7 flag.
pub(super) fn target(
    pos: Position,
    life: LifeState,
    faction: u8,
    stabilized: Option<bool>,
) -> DownedTarget {
    DownedTarget {
        pos,
        life,
        faction: Faction::new(faction),
        stabilized: stabilized.map(Stabilized::new),
    }
}

/// The canonical passing stabilize setup: an Alive ally one cell from a Downed,
/// un-stabilized same-faction target.
pub(super) fn stabilize_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, None);
    (a, t)
}

/// The canonical passing execute setup: an Alive enemy one cell from a Downed
/// opposing-faction target.
pub(super) fn execute_pass() -> (Actor, DownedTarget) {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, None);
    (a, t)
}
