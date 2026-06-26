//! The [`dispatch_end_turn`] system — the turn-cycle engine that drains
//! [`EndTurnRequested`] and advances the active team, regenerating TU at each turn-start
//! (GTW-309).

use bevy::prelude::{Message, MessageReader, MessageWriter, Query, ResMut};

use crate::{
    acts::EndTurnRequested,
    ganger::{Faction, Tu, TuMax},
    turn::{ActiveFaction, regen::regen_team_tu},
};

/// A **turn started** — the combat-log signal that the turn passed to `now_active`
/// (GTW-328), emitted ONCE per [`ActiveFaction`] advance inside [`dispatch_end_turn`].
///
/// The combat-text LOG event for a turn boundary ("Player turn" / "Enemy turn") — the
/// user-facing announcement that the active team changed. Since GTW-70 a single
/// [`EndTurnRequested`] advances [`ActiveFaction`] EXACTLY ONCE — the ending team hands off
/// to the other team and the cycle STOPS there (no auto-pass) — so [`dispatch_end_turn`]
/// emits ONE [`TurnStarted`] per request, announcing the team that just became active. The
/// enemy turn is now driven by the GTW-70 enemy-AI brain ([`crate::ai::enemy_ai_turn`]),
/// which emits its OWN [`EndTurnRequested`] to hand control back to the player when the
/// enemy has nothing left to do. It adds **no** turn-cycle logic and re-resolves nothing —
/// pure exposure of the faction the cycle just made active.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`crate::acts::ReloadResult`]. [`now_active`](TurnStarted::now_active) is the domain
/// [`Faction`] newtype — the combat-log presenter compares it to the
/// [`PlayerFaction`](crate::battle::PlayerFaction) to render "Player turn" vs "Enemy turn".
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

/// **Dispatch an end-turn** — hand the turn to the other team and regenerate that team's
/// TU at its turn-start (GTW-309; GTW-70 removed the enemy auto-pass).
///
/// The turn-cycle engine. Drains [`MessageReader<EndTurnRequested>`] and per message:
///
/// 1. Advances [`ActiveFaction`] to the OTHER team ([`ActiveFaction::advance`] — gang
///    `0` ⇄ `1`), so the turn passes off the team that just ended.
/// 2. Runs the newly-active team's turn-start TU regen ([`regen_team_tu`] — resets [`Tu`]
///    to [`TuMax`] for that team ONLY; the other team's TU is left untouched).
/// 3. **STOPS** — it emits ONE [`TurnStarted`] for the now-active team and does nothing
///    more. There is **no auto-pass** (GTW-70 removed it): when the player ends its turn,
///    control genuinely passes to the ENEMY and stays there. The GTW-70 enemy-AI brain
///    ([`crate::ai::enemy_ai_turn`]) then drives the enemy turn across the following frames
///    and emits its OWN [`EndTurnRequested`] to hand control back to the player when the
///    enemy is done — so the player→enemy→player cycle now takes two end-turn signals (one
///    from the player, one from the enemy brain), not one signal with a double-advance.
///
/// A query/resource system (NO `&mut World` — `bevy-traps.md` #7). It is guarded
/// `.run_if(`[`resource_exists`](bevy::prelude::resource_exists)`::<`[`ActiveFaction`]`>)`,
/// so it is INERT (and its [`ResMut<ActiveFaction>`] read panic-free) outside a live
/// battle — [`ActiveFaction`] shares the
/// [`BattleInProgress`](crate::battle::BattleInProgress) lifetime (`bevy-traps.md` #1).
/// GTW-70 dropped the [`PlayerFaction`](crate::battle::PlayerFaction) param: with the
/// auto-pass gone the cycle no longer needs to know which team is the player — it advances
/// exactly once per request regardless.
pub fn dispatch_end_turn(
    mut requests: MessageReader<EndTurnRequested>,
    mut active: ResMut<ActiveFaction>,
    mut gangers: Query<(&Faction, &mut Tu, &TuMax)>,
    mut turns: MessageWriter<TurnStarted>,
) {
    for _request in requests.read() {
        // 1. Hand the turn to the other team and 2. regenerate ITS TU at turn-start, then
        //    3. STOP — exactly ONE advance per request (GTW-70 removed the enemy auto-pass).
        //    When the player ends its turn, control genuinely passes to the enemy and stays
        //    there; the GTW-70 enemy-AI brain (`enemy_ai_turn`) drives the enemy turn and
        //    emits its own EndTurnRequested to hand control back to the player.
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
    }
}
