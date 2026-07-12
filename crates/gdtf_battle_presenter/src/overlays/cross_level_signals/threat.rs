//! Threat gathering (GTW-596): the fog-gated cross-level enemy scan — reads the
//! SAME free fn [`is_ganger_visible`] [`present_fog`](crate::present_fog) /
//! [`resolve_ganger_visibility`](crate::resolve_ganger_visibility) already use,
//! never a parallel visibility check.

use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, Faction, Level, LifeState, Position},
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

use super::types::LevelDelta;

/// Gather one `(cell, level-delta)` entry per LIVE enemy ganger whose actual
/// `(cell, level)` is squad-VISIBLE and NOT on `active_level` — the load-bearing
/// fog-gating invariant (never leak an UNSEEN / EXPLORED-only enemy).
///
/// `player` mirrors the ganger-visibility resolver's fail-closed default
/// ([`actor_relation`](crate::actors::ganger::GangerVisibilityFacts) via the
/// GTW-627 classifier): with no known [`PlayerFaction`] every ganger is treated as
/// [`FactionRelation::Other`] (never trivially visible), so an unset player
/// faction can never leak a "threat" through the `OwnSquad` branch.
pub(super) fn gather_threats<'a>(
    gangers: impl Iterator<Item = (&'a Position, &'a Faction, &'a LifeState)>,
    squad: &SquadVisibility,
    player: Option<PlayerFaction>,
    active_level: Level,
) -> Vec<(Cell, LevelDelta)> {
    let mut out = Vec::new();
    for (pos, faction, life) in gangers {
        if !*life.is_active() {
            continue; // a downed / dead enemy poses no threat
        }
        let is_player = player.is_some_and(|p| *p == *faction);
        if is_player {
            continue; // own-squad members never surface a "threat" badge
        }
        let key = **pos;
        let (cell, level) = key.split();
        if level == active_level {
            continue; // same-storey — an ordinary sprite draws it, not a badge
        }
        if !*is_ganger_visible(squad, &key, FactionRelation::Other) {
            continue; // fog-gated: never leak an UNSEEN / EXPLORED-only enemy
        }
        out.push((cell, LevelDelta::new(signed_delta(level, active_level))));
    }
    out
}

/// The signed storey delta from `active` to `level` — positive when `level` sits
/// ABOVE `active`. Both are bounded `0..MAX_LEVELS` (8), so the difference always
/// fits an `i8` (`-7..=7`); the `unwrap_or` is a defensive no-panic fallback for
/// an unreachable overflow, never expected to trigger.
fn signed_delta(level: Level, active: Level) -> i8 {
    i8::try_from(i16::from(*level) - i16::from(*active)).unwrap_or(0)
}
