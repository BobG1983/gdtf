//! Emit [`BattleWon`] / [`BattleLost`] when one side is wiped out.

use bevy::prelude::{Local, MessageWriter, Query, Res};

use crate::{
    battle::{
        messages::{BattleLost, BattleWon},
        resources::{BattleRoster, PlayerFaction},
    },
    ganger::{Faction, LifeState},
};

/// Check win/loss conditions once per side and emit the matching message.
pub fn check_outcome(
    mut won: MessageWriter<BattleWon>,
    mut lost: MessageWriter<BattleLost>,
    gangers: Query<(&Faction, &LifeState)>,
    player: Res<PlayerFaction>,
    roster: Res<BattleRoster>,
    mut won_emitted: Local<bool>,
    mut lost_emitted: Local<bool>,
) {
    let player = **player;

    let enemies_fielded = *roster.has_enemy_of(player);
    let players_fielded = *roster.has_player(player);

    let mut any_enemy_up = false;
    let mut any_player_up = false;
    for (&faction, &life) in &gangers {
        if life != LifeState::Alive {
            continue;
        }
        if faction == player {
            any_player_up = true;
        } else {
            any_enemy_up = true;
        }
    }

    if enemies_fielded && !any_enemy_up && any_player_up && !*won_emitted {
        won.write(BattleWon);
        *won_emitted = true;
    }
    if players_fielded && !any_player_up && !*lost_emitted {
        lost.write(BattleLost);
        *lost_emitted = true;
    }
}
