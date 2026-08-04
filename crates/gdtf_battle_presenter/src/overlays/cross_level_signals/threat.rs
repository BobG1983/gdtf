use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, Faction, Level, LifeState, Position},
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

use super::types::LevelDelta;

pub(super) fn gather_threats<'a>(
    gangers: impl Iterator<Item = (&'a Position, &'a Faction, &'a LifeState)>,
    squad: &SquadVisibility,
    player: Option<PlayerFaction>,
    active_level: Level,
) -> Vec<(Cell, LevelDelta)> {
    let mut out = Vec::new();
    for (pos, faction, life) in gangers {
        if !*life.is_active() {
            continue;
        }
        let is_player = player.is_some_and(|p| *p == *faction);
        if is_player {
            continue;
        }
        let key = **pos;
        let (cell, level) = key.split();
        if level == active_level {
            continue;
        }
        if !*is_ganger_visible(squad, &key, FactionRelation::Other) {
            continue;
        }
        out.push((cell, LevelDelta::new(signed_delta(level, active_level))));
    }
    out
}

fn signed_delta(level: Level, active: Level) -> i8 {
    i8::try_from(i16::from(*level) - i16::from(*active)).unwrap_or(0)
}
