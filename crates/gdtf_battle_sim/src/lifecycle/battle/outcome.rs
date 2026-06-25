//! The roster-grounded WIN/LOSS outcome census [`check_outcome`] — the
//! [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate)-band system
//! that emits [`BattleWon`] / [`BattleLost`] (GTW-237).

use bevy::prelude::{Local, MessageWriter, Query, Res};

use crate::{
    battle::{
        messages::{BattleLost, BattleWon},
        resources::{BattleRoster, PlayerFaction},
    },
    ganger::{Faction, LifeState},
};

/// **Check the battle outcome** — emit [`BattleWon`] when every fielded enemy ganger is
/// out of the fight (a player still up), and [`BattleLost`] when every player ganger is
/// out (GTW-237).
///
/// The win/loss conditions, the "out of the fight" = `Downed`/`Dead` rule, the mutual-wipe
/// = loss resolution, and the roster-grounded (not scan-grounded) existence test are
/// design canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`
/// and the "out of the fight" glossary entry. This system is that canon's sim signal.
///
/// The sim's roster-grounded outcome census. A query/resource/local system (NO `&mut
/// World` — `bevy-traps.md` #7). It runs
/// `.in_set(`[`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate)`)`, the
/// band gated on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<`[`BattleInProgress`](crate::battle::BattleInProgress)`>`,
/// so it is INERT outside a live battle and its [`Res<PlayerFaction>`] +
/// [`Res<BattleRoster>`] reads (both battle-lifetime) are panic-free (`bevy-traps.md` #1).
///
/// Two sources, kept separate by design:
///
/// - **Existence** comes from the [`BattleRoster`] (the gangs fielded at setup), NOT a
///   per-frame entity scan: `enemies_fielded = roster.has_enemy_of(*player)`,
///   `players_fielded = roster.has_player(*player)`. This is the bug-2 fix — the win is
///   robust regardless of whether dead gangers are ever despawned.
/// - **Liveness** comes from the live [`Query`]`<(&`[`Faction`]`, &`[`LifeState`]`)>`: a
///   ganger is "up" iff [`LifeState::Alive`] ([`LifeState::Downed`] AND [`LifeState::Dead`]
///   both count OUT, per the incapacitated semantics). `any_enemy_up` =
///   ∃ a non-player Alive ganger; `any_player_up` = ∃ a player Alive ganger.
///
/// Emits [`BattleWon`] iff `enemies_fielded && !any_enemy_up && any_player_up` and not
/// already emitted (then latches its [`Local<bool>`]); emits [`BattleLost`] iff
/// `players_fielded && !any_player_up` and not already emitted (then latches). The two
/// [`Local<bool>`] latches keep each outcome to at most ONE emit per battle (no per-frame
/// spam). Requiring a surviving player for the WIN makes the two outcomes mutually
/// exclusive — a MUTUAL WIPE (last enemy and last player fall together) resolves to
/// [`BattleLost`], not [`BattleWon`]. Player gangers are never inspected for the win; enemy
/// gangers never for the loss.
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

    // Existence from the ROSTER (the gangs fielded at setup), not the live scan.
    let enemies_fielded = roster.has_enemy_of(player);
    let players_fielded = roster.has_player(player);

    // Liveness from the live scan: "up" iff Alive (Downed AND Dead both count OUT).
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

    // WIN: every fielded enemy is out AND a player still stands (so a mutual wipe is a
    // LOSS, not a win) — at most once per battle.
    if enemies_fielded && !any_enemy_up && any_player_up && !*won_emitted {
        won.write(BattleWon);
        *won_emitted = true;
    }
    // LOSS: every player ganger is out — at most once per battle. Enemy liveness is
    // irrelevant: the player losing is a loss whether or not enemies remain.
    if players_fielded && !any_player_up && !*lost_emitted {
        lost.write(BattleLost);
        *lost_emitted = true;
    }
}
