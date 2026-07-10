//! The **enter/exit-emplacement** dispatch systems (GTW-543, child GTW-41c) —
//! [`dispatch_enter_emplacement`] and [`dispatch_exit_emplacement`], which drain the buffered
//! [`EnterEmplacementRequested`] / [`ExitEmplacementRequested`] and, per message, man / dismount
//! ONE weapon emplacement by RE-GATING in the sim and REUSING the GTW-543 toggle mechanism.
//!
//! These MIRROR the open-door dispatch ([`dispatch_open_door`](super::open_door::dispatch_open_door),
//! GTW-315) exactly — the deliberate act's input-seam offer is advisory, so the sim RE-GATES three
//! facts before acting, spends a TU leaf, and writes a
//! [`SetEmplacement`](crate::terrain::emplacement::SetEmplacement) that the GTW-543
//! [`apply_emplacement_toggle`](crate::terrain::emplacement::apply_emplacement_toggle) mechanism
//! resolves one frame later (the documented settle). Neither dispatch flips
//! [`EmplacementState`](crate::terrain::emplacement::EmplacementState) directly.
//!
//! - **Enter**: gate the target is [`EmplacementState::Vacant`], the actor is 8-adjacent + affords
//!   the [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf → spend it + write a
//!   [`SetEmplacement::occupy`](crate::terrain::emplacement::SetEmplacement::occupy). The toggle
//!   then forces the occupant's cover band + spawns the mounted gun.
//! - **Exit**: gate the emplacement's
//!   [`EmplacementOccupant`](crate::terrain::emplacement::EmplacementOccupant) IS the actor + the
//!   actor affords the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu) leaf → spend it +
//!   write a [`SetEmplacement::vacate`](crate::terrain::emplacement::SetEmplacement::vacate). Exit
//!   is a SEPARATE TU-costed action — there is NO force-eject.
//!
//! Both acts draw NO RNG and are deterministic — the same message order yields the same outcome.
//! Param-only (`bevy-traps.md` #7 — no `&mut World`); fail-closed on missing components /
//! resources.

use bevy::prelude::{MessageReader, MessageWriter, Query, Res};

use crate::{
    acts::{
        downed::is_8_adjacent,
        request::{EnterEmplacementRequested, ExitEmplacementRequested},
    },
    ganger::{Position, Tu},
    terrain::{
        emplacement::{EmplacementOccupant, EmplacementState, SetEmplacement},
        entity::TerrainCell,
    },
    tu::spend_tu,
    tuning::CombatTuning,
};

/// **Dispatch** buffered [`EnterEmplacementRequested`] messages — the deliberate ENTER-EMPLACEMENT
/// act (GTW-543), resolved by RE-GATING in the sim then REUSING the GTW-543 occupy mechanism.
///
/// For each request:
///
/// 1. **Gate the emplacement.** Fetch the target's `(&`[`EmplacementState`]`, &`[`TerrainCell`]`)`.
///    A message for a non-emplacement / despawned entity (no [`EmplacementState`]) is skipped
///    (fail-closed). An already-OCCUPIED emplacement is a no-op (no force-eject — a manned mount
///    needs no re-occupy and costs no TU).
/// 2. **Gate the actor.** Fetch the `actor`'s `(&`[`Position`]`, &`[`Tu`]`)`. A missing actor is
///    skipped (fail-closed). The actor must be
///    [`is_8_adjacent`](crate::acts::downed::is_8_adjacent) to the emplacement's cell (the GTW-508
///    structural-melee actor-vs-cell idiom) AND afford the
///    [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf. A rejected gate is a no-op.
/// 3. **Charge + occupy.** On pass, spend the [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu)
///    off the actor's `&mut Tu` (saturating), and write a
///    [`SetEmplacement::occupy`](crate::terrain::emplacement::SetEmplacement::occupy) — the GTW-543
///    mechanism ([`apply_emplacement_toggle`](crate::terrain::emplacement::apply_emplacement_toggle))
///    flips the state, forces the occupant band to HIGH, and spawns the mounted gun the next tick
///    (the documented settle). This act never flips [`EmplacementState`] itself.
///
/// The actor's `&mut Tu` and the emplacement's `&EmplacementState` live on disjoint entities, so
/// the two queries never conflict. Fail-closed: a missing tuning / component mans no emplacement
/// (never a panic). Param-only (`bevy-traps.md` #7).
pub fn dispatch_enter_emplacement(
    mut requests: MessageReader<EnterEmplacementRequested>,
    emplacements: Query<(&EmplacementState, &TerrainCell)>,
    mut actors: Query<(&Position, &mut Tu)>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    // `bevy-traps.md` #1: fail closed (no panic) on the REQUIRED battle-lifetime CombatTuning the
    // enter-TU cost is read from. A focused harness that opens BattleInProgress without it mans no
    // emplacement (no panic).
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        // (1) Gate the emplacement: it must carry an EmplacementState (be an emplacement) AND be
        //     VACANT. A stray message for a non-emplacement / despawned entity, or an
        //     already-occupied emplacement, is a no-op (no force-eject).
        let Ok((state, cell)) = emplacements.get(request.emplacement) else {
            continue;
        };
        if *state.is_occupied() {
            continue;
        }
        // (2) Gate the actor: it must exist (have a Position + Tu), be 8-adjacent to the
        //     emplacement's cell, and afford the EnterEmplacementTu leaf. The 8-adjacency reuses
        //     `is_8_adjacent` over the actor's Position vs a Position AT the emplacement cell (the
        //     GTW-508 actor-vs-cell idiom). Same-level Chebyshev-1 (incl. diagonals).
        let Ok((&actor_pos, mut actor_tu)) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.enter_emplacement_tu);
        if !*is_8_adjacent(actor_pos, Position::new(**cell)) || **actor_tu < *cost {
            continue;
        }
        // (3) Charge + occupy. Spend the EnterEmplacementTu (saturating), then man the emplacement
        //     through the GTW-543 mechanism (never EmplacementState directly).
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetEmplacement::occupy(request.emplacement, request.actor));
    }
}

/// **Dispatch** buffered [`ExitEmplacementRequested`] messages — the deliberate EXIT-EMPLACEMENT
/// act (GTW-543), resolved by RE-GATING in the sim then REUSING the GTW-543 vacate mechanism.
///
/// For each request:
///
/// 1. **Gate the occupancy.** Fetch the emplacement's `(&`[`EmplacementState`]`,
///    &`[`EmplacementOccupant`]`)`. A message for an entity that is not an OCCUPIED emplacement (no
///    occupant recorded) is skipped (fail-closed). The recorded
///    [`EmplacementOccupant`](crate::terrain::emplacement::EmplacementOccupant) MUST BE the
///    `actor` — a ganger can only dismount the emplacement IT mans (no dismounting another
///    occupant), and there is NO force-eject.
/// 2. **Gate the actor.** Fetch the `actor`'s `&mut `[`Tu`] and require it affords the
///    [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu) leaf. A rejected gate is a no-op (no
///    TU, no vacate) — the ganger stays seated until it can afford to dismount.
/// 3. **Charge + vacate.** On pass, spend the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu)
///    off the actor's `&mut Tu` (saturating), and write a
///    [`SetEmplacement::vacate`](crate::terrain::emplacement::SetEmplacement::vacate) — the toggle
///    restores the occupant's stance band + despawns the mounted gun the next tick. This act never
///    flips [`EmplacementState`] itself.
///
/// Fail-closed: a missing tuning / component dismounts no one (never a panic). Param-only
/// (`bevy-traps.md` #7).
pub fn dispatch_exit_emplacement(
    mut requests: MessageReader<ExitEmplacementRequested>,
    emplacements: Query<(&EmplacementState, &EmplacementOccupant)>,
    mut actors: Query<&mut Tu>,
    tuning: Option<Res<CombatTuning>>,
    mut toggles: MessageWriter<SetEmplacement>,
) {
    // `bevy-traps.md` #1: fail closed on the required battle-lifetime CombatTuning the exit-TU cost
    // is read from.
    let Some(tuning) = tuning else {
        return;
    };
    for request in requests.read() {
        // (1) Gate the occupancy: the emplacement must be OCCUPIED and its recorded occupant must
        //     be the requesting actor (a ganger can only dismount the emplacement it mans).
        let Ok((state, occupant)) = emplacements.get(request.emplacement) else {
            continue;
        };
        if !*state.is_occupied() || **occupant != request.actor {
            continue;
        }
        // (2) Gate the actor: it must exist (have Tu) and afford the ExitEmplacementTu leaf.
        let Ok(mut actor_tu) = actors.get_mut(request.actor) else {
            continue;
        };
        let cost = Tu::new(*tuning.exit_emplacement_tu);
        if **actor_tu < *cost {
            continue;
        }
        // (3) Charge + vacate. Spend the ExitEmplacementTu (saturating), then dismount through the
        //     GTW-543 mechanism (never EmplacementState directly).
        spend_tu(&mut actor_tu, cost);
        toggles.write(SetEmplacement::vacate(request.emplacement, request.actor));
    }
}
