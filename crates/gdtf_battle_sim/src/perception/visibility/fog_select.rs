//! The AI MOVE-fog resource [`OmniscientFog`] and the shared [`move_fog`] selector — the
//! ONE move-planning fog both the executor ([`dispatch_move`](crate::acts::dispatch_move))
//! and the planner (the enemy AI, [`crate::ai`]) read, so the two can never drift (GTW-70).
//!
//! The blocker GTW-70 closes: before it, `dispatch_move` planned over the PLAYER squad fog
//! while the AI planned over an omniscient view — two different gates, so an AI reposition
//! into player-unseen space routed for the planner yet `MoveRejected(Unreachable)` for the
//! executor (no TU spent, infinite re-emit). The fix is ONE selector both call: a
//! mover-faction-keyed choice between the player fog and the battle-lifetime omniscient
//! fog. A symmetric enemy fog-of-war (planning on last-seen positions) is **deferred to
//! GTW-71** — this slice deliberately ships an omniscient move fog shared by both sides of
//! the move gate.

use bevy::prelude::{Deref, Resource};

use crate::{ganger::Faction, visibility::SquadVisibility};

/// The battle-lifetime **omniscient move fog** — the AI's move-planning fog, a
/// [`SquadVisibility`] with EVERY in-bounds cell both VISIBLE and EXPLORED (GTW-70).
///
/// A named newtype [`Resource`] over [`SquadVisibility`] (no-bare-types: the AI move fog
/// is a distinct domain value from the player squad fog, even over the same inner type —
/// the [`ActiveFaction`](crate::turn::ActiveFaction)-vs-[`Faction`] precedent). The derived
/// [`Deref`] reads the inner [`SquadVisibility`] back; the inner is PRIVATE, built only
/// through [`OmniscientFog::new`] from
/// [`SquadVisibility::omniscient`](crate::visibility::SquadVisibility::omniscient).
///
/// **Lifetime tracks [`BattleInProgress`](crate::battle::BattleInProgress):** the battle
/// setup inserts it on the same successful-setup `Ok` path that inserts the player
/// [`SquadVisibility`] (built once from the grid extent) and the teardown removes it
/// alongside — so it is present for exactly the battle-active window. Every reader
/// ([`dispatch_move`](crate::acts::dispatch_move) via [`move_fog`], the enemy AI) takes it
/// as `Option<Res<_>>` (or gates on the battle witness) so it is panic-free outside a live
/// battle (`bevy-traps.md` #1).
///
/// It is the MOVE planner's fog ONLY — it never lets the AI SHOOT. Firing gates on the
/// real per-pair [`can_see`](crate::los::can_see); this fog merely lets the AI navigate
/// toward the player's true cell while colliding correctly (every occupant blocks, since
/// every cell is VISIBLE under it).
#[derive(Resource, Deref, Debug, Clone, PartialEq, Eq)]
pub struct OmniscientFog(SquadVisibility);

impl OmniscientFog {
    /// Build the omniscient move-fog resource from an omniscient
    /// [`SquadVisibility`](crate::visibility::SquadVisibility::omniscient).
    #[must_use]
    pub const fn new(fog: SquadVisibility) -> Self {
        Self(fog)
    }
}

/// Select the move-planning fog for a mover of `mover_faction` — the player squad fog when
/// the mover IS the player, else the omniscient fog (GTW-70).
///
/// The ONE selector both the move EXECUTOR ([`dispatch_move`](crate::acts::dispatch_move))
/// and the move PLANNER (the enemy AI) call, so planner and executor read the IDENTICAL
/// fog for any given mover and cannot drift:
///
/// - **Player mover** (`mover_faction == player_faction`) → `player_fog`: player movement
///   stays identical to the GTW-353 visibility-gated routing (plan only through what
///   the squad has seen).
/// - **Non-player mover** → `omniscient`: an enemy navigates toward the player's TRUE cell
///   through routable space, gated only by real geometry/occupancy, never by the *player's*
///   fog (which would soft-lock it on a player-unseen reposition — the blocker this fixes).
///
/// Pure: a borrow-returning faction comparison, no world access. The two fog borrows share
/// the caller's lifetime `'a`.
#[must_use]
pub fn move_fog<'a>(
    mover_faction: Faction,
    player_faction: Faction,
    player_fog: &'a SquadVisibility,
    omniscient: &'a SquadVisibility,
) -> &'a SquadVisibility {
    if mover_faction == player_faction {
        player_fog
    } else {
        omniscient
    }
}
