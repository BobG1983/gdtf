//! The **reload** dispatch — drain each buffered [`ReloadRequested`] message and
//! refill the actor's magazine, charging the weapon's per-weapon reload TU cost
//! (GTW-275).
//!
//! The real, TU-costed `ReloadAct`: the cost is the actor's OWN
//! [`Magazine::reload_tu`](crate::magazine::Magazine::reload_tu) (a per-WEAPON number
//! authored in the weapon `.ron`, USER DIRECTIVE 2026-06-17 — NOT a global tuning
//! leaf). It mirrors the [`set_stance`](crate::posture::set_stance) /
//! [`set_facing`](crate::posture::set_facing) pattern: GATE first (alive +
//! affordable), then [`spend_tu`](crate::tu::spend_tu) and
//! [`refill`](crate::magazine::Magazine::refill). It fetches the actor's components via
//! a Bevy query (`bevy-traps.md` #7 — no `&mut World`).

use bevy::prelude::{MessageReader, Query};

use crate::{
    acts::request::ReloadRequested,
    ganger::{LifeState, Tu},
    magazine::Magazine,
    tu::{can_spend_tu, spend_tu},
};

/// **Dispatch** buffered [`ReloadRequested`] messages — drain each and reload the
/// actor's magazine, charging the weapon's per-weapon reload TU cost (GTW-275).
///
/// Queries the actor's `(&mut `[`Magazine`]`, &mut `[`Tu`]`, &`[`LifeState`]`)` and:
///
/// 1. GATES like the other acts — skips (silently, no panic) when the actor is not
///    [`Alive`](crate::ganger::LifeState::Alive), or when its [`Tu`] cannot afford the
///    magazine's own [`reload_tu`](crate::magazine::Magazine::reload_tu)
///    ([`can_spend_tu`]); the rejection is silent, matching the fire / move / stance
///    acts.
/// 2. On success, [`spend_tu`]s the magazine's `reload_tu` then
///    [`refill`](crate::magazine::Magazine::refill)s the magazine to full
///    (`loaded == size`).
///
/// **Already-full behavior (engineer's call, FLAGGED, GTW-275 AC3): a reload of an
/// already-FULL magazine is a NO-OP — no TU is charged.** This mirrors the codebase's
/// "redundant act = no charge" convention ([`set_stance`](crate::posture::set_stance)
/// charges only on a real stance change; [`set_facing`](crate::posture::set_facing)
/// only on a real turn) and is the kinder UX (a misclick on a full magazine is free).
/// The ONLY designed cost is the `reload_tu` spend — no turn-ending semantics are
/// invented.
///
/// REUSES the magazine grouping's own primitives. A message for an actor missing any
/// queried component is skipped (fail-closed, no panic — the `dispatch_set_*`
/// precedent).
pub fn dispatch_reload(
    mut requests: MessageReader<ReloadRequested>,
    mut actors: Query<(&'static mut Magazine, &'static mut Tu, &'static LifeState)>,
) {
    for request in requests.read() {
        let Ok((mut magazine, mut tu, &life)) = actors.get_mut(request.actor) else {
            continue;
        };

        // GATE: alive only (a Downed/Dead ganger cannot reload), mirroring the firing
        // act's liveness gate.
        if life != LifeState::Alive {
            continue;
        }

        // Already-full reload is a no-op — no charge (FLAGGED choice; see the fn doc).
        if magazine.is_full() {
            continue;
        }

        // GATE: affordable — the per-weapon reload_tu must be payable (silent reject
        // when short, matching fire/move/stance).
        let cost = Tu::new(*magazine.reload_tu());
        if !can_spend_tu(&tu, cost) {
            continue;
        }

        // SUCCESS: charge the reload_tu, then refill the magazine to full.
        spend_tu(&mut tu, cost);
        magazine.refill();
    }
}
