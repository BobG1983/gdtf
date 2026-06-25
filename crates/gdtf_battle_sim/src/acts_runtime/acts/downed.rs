//! The **from-Downed** dispatch systems — drain each buffered downed-act `*Requested`
//! message and run the landed faction-gated verb once per message (E10.2 AC5).
//!
//! No act logic is reimplemented: each system assembles the
//! [`Actor`] / [`DownedTarget`] bundles from the queried components and REUSES the landed
//! [`stabilize_downed`] / [`execute_downed`] verb verbatim — the faction gate
//! ([`can_stabilize`](crate::downed_acts::can_stabilize) /
//! [`can_execute`](crate::downed_acts::can_execute)) holds end-to-end. Fetches via Bevy
//! queries (`bevy-traps.md` #7 — no `&mut World`).

use bevy::prelude::{MessageReader, Query, Res};

use crate::{
    acts::request::{ExecuteDownedRequested, StabilizeDownedRequested},
    downed_acts::{Actor, DownedTarget, execute_downed, stabilize_downed},
    ganger::{Faction, LifeState, Position, Stabilized},
    tuning::CombatTuning,
};

/// The downed-act read shape — the [`Position`] / [`LifeState`] / [`Faction`] every
/// from-Downed verb gates on (plus the target's [`Stabilized`] flag), queried off both
/// the actor and the target entity.
///
/// A type alias for the read tuple shared by [`dispatch_stabilize_downed`] /
/// [`dispatch_execute_downed`] so each system's signature stays readable: the [`Actor`] /
/// [`DownedTarget`] bundles ([`downed_acts`](crate::downed_acts)) are assembled from these
/// reads at the call site. The target's [`Stabilized`] is `Option` (it may be absent —
/// `None` is not-yet-stabilized). [`LifeState`] is `&mut` only on the target (execute
/// transitions it); the read shape is the same tuple for both actor and target via
/// `get`/`get_mut`.
type DownedReads<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static mut LifeState,
        &'static Faction,
        Option<&'static mut Stabilized>,
    ),
>;

/// **Dispatch** buffered [`StabilizeDownedRequested`] messages — drain each and run the
/// landed [`stabilize_downed`] verb once per message (E10.2 AC5).
///
/// Reads the actor's [`Position`] / [`LifeState`] / [`Faction`] and the target's same
/// trio + [`Stabilized`] flag, assembles the [`Actor`] / [`DownedTarget`] bundles
/// ([`downed_acts`](crate::downed_acts)), and calls [`stabilize_downed`] — whose
/// faction gate ([`can_stabilize`](crate::downed_acts::can_stabilize)) holds end-to-end
/// (a cross-faction enemy is a no-op). On success it SETS the target's [`Stabilized`]
/// flag (the target stays [`LifeState::Downed`] — the verb never writes its life state).
/// REUSES the landed verb verbatim; an actor / target missing the read components or the
/// target missing its [`Stabilized`] component is skipped (fail-closed, no panic).
///
/// The actor and target reads are taken as snapshots (the gating reads are `Copy`), so
/// the verb's `&mut Stabilized` write to the target does not overlap a live read borrow.
pub fn dispatch_stabilize_downed(
    mut requests: MessageReader<StabilizeDownedRequested>,
    mut gangers: DownedReads,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        // Snapshot the actor's gating reads (Copy newtypes), releasing the read borrow
        // before the target's &mut Stabilized write.
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        // Snapshot the target's gating reads + its current Stabilized flag.
        let Ok((&target_pos, &target_life, &target_faction, target_stab)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:        target_pos,
            life:       target_life,
            faction:    target_faction,
            stabilized: target_stab.copied(),
        };
        // Re-fetch the target's &mut Stabilized to apply the verb's write (the snapshot
        // borrows above are released — get_mut takes a fresh exclusive borrow).
        let Ok((_, _, _, Some(mut flag))) = gangers.get_mut(request.target) else {
            continue;
        };
        stabilize_downed(&actor, &target, &mut flag, &tuning);
    }
}

/// **Dispatch** buffered [`ExecuteDownedRequested`] messages — drain each and run the
/// landed [`execute_downed`] verb once per message (E10.2 AC5).
///
/// Reads the actor's and target's [`Position`] / [`LifeState`] / [`Faction`], assembles
/// the [`Actor`] / [`DownedTarget`] bundles ([`downed_acts`](crate::downed_acts)), and
/// calls [`execute_downed`] — whose faction gate
/// ([`can_execute`](crate::downed_acts::can_execute)) holds end-to-end (a same-faction
/// ally is a no-op). On success it transitions the target to [`LifeState::Dead`]. REUSES
/// the landed verb verbatim; an actor / target missing the read components is skipped
/// (fail-closed, no panic).
///
/// The actor and target gating reads are snapshotted (Copy), so the verb's `&mut
/// LifeState` write to the target does not overlap a live read borrow.
pub fn dispatch_execute_downed(
    mut requests: MessageReader<ExecuteDownedRequested>,
    mut gangers: DownedReads,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((&actor_pos, &actor_life, &actor_faction, _)) = gangers.get(request.actor) else {
            continue;
        };
        let Ok((&target_pos, &target_life, &target_faction, target_stab)) =
            gangers.get(request.target)
        else {
            continue;
        };
        let actor = Actor {
            pos:     actor_pos,
            life:    actor_life,
            faction: actor_faction,
        };
        let target = DownedTarget {
            pos:        target_pos,
            life:       target_life,
            faction:    target_faction,
            stabilized: target_stab.copied(),
        };
        // Re-fetch the target's &mut LifeState to apply the verb's write.
        let Ok((_, mut life, ..)) = gangers.get_mut(request.target) else {
            continue;
        };
        execute_downed(&actor, &target, &mut life, &tuning);
    }
}
