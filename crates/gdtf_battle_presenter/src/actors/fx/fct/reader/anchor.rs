//! The pop anchor resolution — struck-ganger position vs the round's impact cell.

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, Level, Position, ShotFired, ShotKind};

/// The `(cell, level)` a round's pops anchor over.
///
/// For a [`ShotKind::Ganger`] outcome it reads the struck ganger's current
/// [`Position`](gdtf_battle_sim::Position) (so the pops sit on the body that was hit, even if
/// that body has since moved off the impact cell) — decomposed via the canonical
/// [`CellLevel::split`](gdtf_battle_sim::CellLevel::split) (GTW-565). For any other kind
/// (cover / slab / ground / miss), or a ganger whose [`Position`] is missing (fail-closed),
/// it falls back to the round's impact `(cell, level)` — where the round landed.
///
/// `pub(in crate::actors::fx)`: called by
/// [`spawn_shot_projectiles`](super::super::super::spawn_shot_projectiles)
/// at projectile-spawn time so the anchor is captured AT THE SHOT and threaded through the
/// staggered projectile → impact pipeline (so the pop still sits on the body that was hit even
/// if it has moved by the time the staggered impact lands).
pub(in crate::actors::fx) fn anchor_cell(
    msg: &ShotFired,
    positions: &Query<&Position>,
) -> (Cell, Level) {
    if let ShotKind::Ganger(entity) = msg.kind
        && let Ok(pos) = positions.get(entity)
    {
        // The canonical CellLevel::split decompose through Position's deref (GTW-565).
        return pos.split();
    }
    (msg.impact_cell, msg.impact_level)
}
