//! The **open-door dispatch** system (GTW-315) — [`dispatch_open_door`], which drains the
//! buffered [`OpenDoorRequested`] and, per message, opens ONE adjacent CLOSED door by
//! RE-GATING in the sim and REUSING the GTW-503 open mechanism.
//!
//! The deliberate OPEN-DOOR act (the player-only contextual Open-Door press, F4) re-gates
//! three facts — the actor exists + can afford the [`OpenDoorTu`](crate::tuning::OpenDoorTu)
//! leaf, and the `door` entity carries an [`OpenState`] that is CLOSED and is 8-adjacent to the
//! actor — then, on pass, spends the [`OpenDoorTu`](crate::tuning::OpenDoorTu) off the actor's TU
//! pool and writes a [`SetOpenable::toggle`] for the door. The door's actual open/close flip +
//! the path / vision unblock are the GTW-503 [`apply_openable_toggle`] mechanism's job (this act
//! NEVER flips [`OpenState`] directly) — so the door clears through the existing GTW-501 /
//! GTW-502 change-detection one frame later (the documented settle).
//!
//! Opening a door draws NO RNG and is deterministic — the same message order yields the same
//! outcome. Param-only (`bevy-traps.md` #7 — no `&mut World`); fail-closed on missing components
//! / resources.

use bevy::prelude::{MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::{downed::is_8_adjacent, request::OpenDoorRequested},
    ganger::{Position, Tu},
    terrain::{
        entity::TerrainCell,
        openable::{OpenState, SetOpenable},
    },
    tu::spend_tu,
    tuning::CombatTuning,
};

/// **Dispatch** buffered [`OpenDoorRequested`] messages — the deliberate OPEN-DOOR act (GTW-315),
/// resolved by RE-GATING in the sim then REUSING the GTW-503 open mechanism.
///
/// For each request:
///
/// 1. **Gate the door.** Fetch the `door`'s `(&`[`OpenState`]`, &`[`TerrainCell`]`)`. A message for
///    a non-openable / despawned entity (no [`OpenState`]) is skipped (fail-closed). A door that
///    is already OPEN is a no-op — the button only OPENS, closing is not offered (F2/detect), so
///    an already-open door needs no toggle and costs no TU.
/// 2. **Gate the actor.** Fetch the `actor`'s `(&`[`Position`]`, &`[`Tu`]`)`. A missing actor (no
///    [`Position`] / [`Tu`]) is skipped (fail-closed). The actor must be
///    [`is_8_adjacent`](crate::acts::downed::is_8_adjacent) to the door's cell (its [`Position`]
///    vs a [`Position`] AT the door's [`TerrainCell`] — the GTW-508 structural-melee adjacency
///    idiom) AND afford the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf. A rejected gate is a
///    no-op (no TU, no toggle).
/// 3. **Charge + toggle.** On pass, spend the [`OpenDoorTu`](crate::tuning::OpenDoorTu) off the
///    actor's `&mut Tu` (saturating), and write a [`SetOpenable::toggle`] for the door — the
///    GTW-503 mechanism ([`apply_openable_toggle`](crate::terrain::openable::apply_openable_toggle))
///    flips [`OpenState`] and drives the GTW-501 / GTW-502 unblock (the documented one-frame
///    settle). This act never flips [`OpenState`] itself.
///
/// The actor's `&mut Tu` and the door's `&OpenState` live on disjoint entities, so the two
/// queries never conflict (a ganger has no `OpenState`; a door has no `Tu`). Fail-closed: a
/// missing tuning / component simply opens no door (never a panic). Param-only
/// (`bevy-traps.md` #7).
pub fn dispatch_open_door(
    mut requests: MessageReader<OpenDoorRequested>,
    doors: Query<(&OpenState, &TerrainCell)>,
    mut actors: Query<(&Position, &mut Tu)>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetOpenable>,
) {
    // `bevy-traps.md` #1: fail closed (no panic) on the REQUIRED battle-lifetime CombatTuning the
    // open-door TU cost is read from. It is sim-set at setup; a focused harness that opens
    // BattleInProgress without it opens no door (no panic).
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        // (1) Gate the door: it must carry an OpenState (be openable) AND be CLOSED. A stray
        //     message for a non-openable / despawned entity, or an already-open door, is a no-op.
        let Ok((open_state, door_cell)) = doors.get(request.door) else {
            continue;
        };
        if *open_state.is_open() {
            // The button only OPENS; an already-open door needs no toggle and costs no TU.
            continue;
        }
        // (2) Gate the actor: it must exist (have a Position + Tu), be 8-adjacent to the door's
        //     cell, and afford the OpenDoorTu leaf. The 8-adjacency reuses `is_8_adjacent` over the
        //     actor's Position vs a Position AT the door cell (the GTW-508 structural-melee idiom —
        //     one adjacency helper for actor-vs-cell reach). Same-level Chebyshev-1 (incl. diagonals).
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.open_door_tu);
        if !*is_8_adjacent(actor_pos, Position::new(**door_cell)) || **actor_tu < *cost {
            continue;
        }
        // (3) Charge + toggle. Spend the OpenDoorTu off the actor (saturating), then flip the door
        //     open through the GTW-503 mechanism (never OpenState directly). The `.toggle` reads the
        //     current (Closed) state — gated Closed above — so it always resolves to Open here.
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetOpenable::toggle(request.door, *open_state));
    }
}
