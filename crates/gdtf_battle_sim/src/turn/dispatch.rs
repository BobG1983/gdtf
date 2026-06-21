//! The [`dispatch_end_turn`] system — the turn-cycle engine that drains
//! [`EndTurnRequested`] and advances the active team, regenerating TU at each turn-start
//! (GTW-309).

use bevy::prelude::{Message, MessageReader, MessageWriter, Query, Res, ResMut};

use crate::{
    acts::EndTurnRequested,
    battle::PlayerFaction,
    ganger::{Faction, Tu, TuMax},
    turn::{ActiveFaction, regen::regen_team_tu},
};

/// A **turn started** — the combat-log signal that the turn passed to `now_active`
/// (GTW-328), emitted ONCE per [`ActiveFaction`] advance inside [`dispatch_end_turn`].
///
/// The combat-text LOG event for a turn boundary ("Player turn" / "Enemy turn") — the
/// user-facing announcement that the active team changed. Because a single
/// [`EndTurnRequested`] cycles the turn off the ending team to the other team AND (while
/// the enemy has no AI) auto-passes back to the player, [`dispatch_end_turn`] advances
/// [`ActiveFaction`] up to TWICE per request — so it emits a [`TurnStarted`] after EACH
/// advance (the enemy turn start, then the player turn start), in order, so the log reads
/// both boundaries. It adds **no** turn-cycle logic and re-resolves nothing — pure
/// exposure of the faction the cycle just made active.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`crate::acts::ReloadResult`]. [`now_active`](TurnStarted::now_active) is the domain
/// [`Faction`] newtype — the combat-log presenter compares it to the
/// [`PlayerFaction`] to render "Player turn" vs "Enemy turn".
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurnStarted {
    /// The faction whose turn just started — the team [`ActiveFaction`] now points at
    /// after the advance.
    pub now_active: Faction,
}

impl TurnStarted {
    /// Build a turn-started signal for the team that just became active.
    #[must_use]
    pub const fn new(now_active: Faction) -> Self {
        Self { now_active }
    }
}

/// **Dispatch an end-turn** — hand the turn to the other team, regenerate that team's TU
/// at its turn-start, and (while the enemy has no AI) auto-pass the enemy turn back to the
/// player (GTW-309).
///
/// The turn-cycle engine. Drains [`MessageReader<EndTurnRequested>`] and per message:
///
/// 1. Advances [`ActiveFaction`] to the OTHER team ([`ActiveFaction::advance`] — gang
///    `0` ⇄ `1`), so the turn passes off the team that just ended.
/// 2. Runs the newly-active team's turn-start TU regen ([`regen_team_tu`] — resets [`Tu`]
///    to [`TuMax`] for that team ONLY; the other team's TU is left untouched).
/// 3. If the newly-active team is the ENEMY (not the [`PlayerFaction`]), AUTO-PASSES: it
///    immediately advances [`ActiveFaction`] back to the player and runs the player's
///    turn-start regen, so control returns to the player. While the enemy has no AI, the
///    enemy turn is a no-op pass; the [`TODO(AI)`] seam below marks where the enemy AI turn
///    replaces it.
///
/// The end state is always [`ActiveFaction`] == the [`PlayerFaction`] (control returns to
/// the player), since the player's own turn ending hands off to the enemy, who auto-passes
/// straight back.
///
/// A query/resource system (NO `&mut World` — `bevy-traps.md` #7). It is guarded
/// `.run_if(`[`resource_exists`](bevy::prelude::resource_exists)`::<`[`ActiveFaction`]`>)`,
/// so it is INERT (and its [`ResMut<ActiveFaction>`] read panic-free) outside a live
/// battle — [`ActiveFaction`] shares the
/// [`BattleInProgress`](crate::battle::BattleInProgress) lifetime (`bevy-traps.md` #1).
/// [`PlayerFaction`] is read as `Option<Res<_>>` for the same reason: it is battle-lifetime
/// (co-inserted with [`ActiveFaction`]), so a missing one (no live battle) makes this a
/// total no-op rather than a panic.
pub fn dispatch_end_turn(
    mut requests: MessageReader<EndTurnRequested>,
    mut active: ResMut<ActiveFaction>,
    player: Option<Res<PlayerFaction>>,
    mut gangers: Query<(&Faction, &mut Tu, &TuMax)>,
    mut turns: MessageWriter<TurnStarted>,
) {
    // PlayerFaction shares ActiveFaction's battle lifetime; without it there is no live
    // battle to cycle, so a missing resource is a total no-op (bevy-traps #1).
    let Some(player) = player else {
        return;
    };
    let player = **player;

    for _request in requests.read() {
        // 1. Hand the turn to the other team and 2. regenerate ITS TU at turn-start.
        active.advance();
        let now_active = **active;
        regen_team_tu(
            gangers
                .iter_mut()
                .map(|(faction, tu, tu_max)| (faction, tu.into_inner(), tu_max)),
            now_active,
        );
        // GTW-328: announce the turn boundary the cycle just crossed (combat-log signal).
        turns.write(TurnStarted::new(now_active));

        // 3. If the newly-active team is the ENEMY, auto-pass the enemy turn straight back
        //    to the player and regenerate the player's TU at its turn-start, so control
        //    returns to the player.
        if now_active != player {
            // TODO(AI): replace auto-pass with enemy AI turn
            active.advance();
            let player_team = **active;
            regen_team_tu(
                gangers
                    .iter_mut()
                    .map(|(faction, tu, tu_max)| (faction, tu.into_inner(), tu_max)),
                player_team,
            );
            // GTW-328: announce the player's turn start after the enemy auto-pass.
            turns.write(TurnStarted::new(player_team));
        }
    }
}
