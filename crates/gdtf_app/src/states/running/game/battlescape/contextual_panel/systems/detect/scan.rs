//! The pure **offer scans** the contextual-panel brain
//! ([`detect_contextual_targets`](super::detect_contextual_targets)) runs to decide what each
//! contextual act should OFFER (GTW-294 / GTW-507 / GTW-508 / GTW-525 — split out of `detect/mod.rs`
//! to keep each concern under the code-health size cap):
//!
//! - [`scan_targets`] — the Execute / Stabilize downed-neighbour scan (GTW-294).
//! - [`scan_melee_target`] — the GTW-507 melee-vs-ganger LOS scan.
//! - [`scan_melee_structure`] — the GTW-508 melee-vs-adjacent-structure (cover-smash) scan.
//! - [`scan_shove_target`] — the GTW-525 shove-vs-ganger scan (no LOS, no weapon).
//! - [`scan_open_door`] — the GTW-315 open-door-vs-adjacent-CLOSED-door scan.
//! - [`scan_enter_emplacement`] — the GTW-543 enter-vs-adjacent-VACANT-emplacement scan.
//! - [`scan_exit_emplacement`] — the GTW-543 exit-vs-occupied-by-the-selection scan.
//!
//! Every scan is PURE over the queried components + the read grids (no world mutation), so the
//! offer logic is testable in isolation from the visibility toggling. The actual sim gates
//! ([`execute_downed`](gdtf_battle_sim::execute_downed) /
//! [`stabilize_downed`](gdtf_battle_sim::stabilize_downed) /
//! [`dispatch_melee`](gdtf_battle_sim::dispatch_melee)) re-check faction + reach (+ LOS for the
//! ganger melee) authoritatively when the act fires; this layer only decides what to OFFER (and
//! reuses [`has_los`] / [`is_8_adjacent`] verbatim so the offers match the sim's geometry truth).

use bevy::prelude::*;
use gdtf_battle_sim::{
    Cell, CellLevel, Level,
    downed_acts::is_8_adjacent,
    ganger::{Facing, Faction, LifeState, Position, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, has_los},
};

use super::{CandidateReads, DoorReads, EmplacementReads, LosGrids};

/// Scans `candidates` for the actor's actionable downed neighbours.
///
/// Returns `(execute, stabilize)`: the first 8-adjacent downed ENEMY (different faction) and
/// the first 8-adjacent downed ALLY (same faction) that is not already
/// [`Stabilized`](gdtf_battle_sim::ganger::Stabilized). Pure over the queried components (no
/// world mutation) so the offer logic is testable in isolation from the visibility toggling.
pub(super) fn scan_targets(
    actor_pos: Position,
    actor_faction: Faction,
    candidates: &Query<CandidateReads>,
) -> (Option<Entity>, Option<Entity>) {
    let mut execute = None;
    let mut stabilize = None;

    for (entity, pos, life, faction, stabilized, _stance) in candidates {
        // Only DOWNED neighbours within the 8-adjacent reach are candidates.
        if *life != LifeState::Downed || !is_8_adjacent(actor_pos, *pos) {
            continue;
        }
        if *faction == actor_faction {
            // A downed ALLY — stabilize, unless its bleed clock is already halted.
            let already_stabilized = stabilized.is_some_and(|flag| **flag);
            if stabilize.is_none() && !already_stabilized {
                stabilize = Some(entity);
            }
        } else if execute.is_none() {
            // A downed ENEMY — execute.
            execute = Some(entity);
        }
    }

    (execute, stabilize)
}

/// Scans `candidates` for the actor's actionable MELEE target — the first 8-adjacent, ALIVE,
/// ENEMY ganger with a clear line of sight from the actor (GTW-507; `docs/combat/resolution.md`
/// §7).
///
/// A STRONGER offer gate than Execute's downed-adjacency: the target must be ALIVE (incl.
/// Downed — [`LifeState::is_active`]), of the OPPOSING faction, 8-adjacent ([`is_8_adjacent`]),
/// AND in clear LOS ([`has_los`], REUSED verbatim so the offered shot matches the sim's geometry
/// truth). Returns the first such candidate, or [`None`] when none qualifies — including when
/// the battle grids are absent (a pre-battle frame), so the melee button stays hidden until a
/// live battle. A corpse never blocks the LOS march (the `is_dead` pass-through reused).
///
/// Pure over the queried components + the read grids (no world mutation), so it is testable in
/// isolation. The sim's [`dispatch_melee`](gdtf_battle_sim::dispatch_melee) gate is the
/// authoritative re-check when the act actually fires; this only decides what to OFFER.
pub(super) fn scan_melee_target(
    actor_pos: Position,
    actor_faction: Faction,
    actor_stance: Stance,
    actor_facing: Facing,
    candidates: &Query<CandidateReads>,
    grids: &LosGrids,
) -> Option<Entity> {
    // The melee LOS gate needs the live battle grids; with any absent (pre-battle) offer nothing
    // — fail-closed (bevy-traps.md #1: handle the Option, never unwrap).
    let (Some(occupancy), Some(surface), Some(cover), Some(tuning)) = (
        grids.occupancy.as_ref(),
        grids.surface.as_ref(),
        grids.cover.as_ref(),
        grids.tuning.as_ref(),
    ) else {
        return None;
    };

    // A Dead ganger is a corpse the LOS march flies THROUGH (the same `is_dead` pass-through
    // `has_los` takes). Read each candidate's CURRENT LifeState; an absent entity is no corpse.
    let is_dead = |entity: Entity| {
        candidates
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };

    // The fallback silhouette for a candidate carrying no stance (a real fielded ganger always
    // has one; this keeps the LOS aim defined for a minimal test/edge entity).
    let standing = Stance::new(StanceKind::Standing);
    for (entity, pos, life, faction, _stabilized, stance) in candidates {
        // An ALIVE (incl. Downed) OPPOSING ganger within the 8-adjacent reach.
        if !life.is_active() || *faction == actor_faction || !is_8_adjacent(actor_pos, *pos) {
            continue;
        }
        // The LOS gate — a clear sight line actor → target over the SAME voxel geometry the sim
        // fires through (built exactly as the sim's `dispatch_melee` builds the observer/target).
        let observer = Observer {
            position:         &actor_pos,
            stance:           &actor_stance,
            facing:           &actor_facing,
            stair_eye_offset: occupancy.stair_eye_offset_at(&actor_pos),
            peek_offset:      PeekOffset::default(),
        };
        let target = Target {
            position: pos,
            stance:   stance.unwrap_or(&standing),
        };
        if *has_los(
            &observer, &target, occupancy, surface, cover, tuning, is_dead,
        ) {
            return Some(entity);
        }
    }
    None
}

/// Scans the actor's 8 same-storey neighbour cells for an intact Cover / Wall cell to SMASH —
/// the first 8-adjacent cell whose [`CoverLedger`](gdtf_battle_sim::CoverLedger) entry stands
/// (not destroyed) (GTW-508; `docs/combat/resolution.md` §7 — the melee-smash of adjacent cover).
///
/// Returns the first such adjacent [`CellLevel`], or [`None`] when no neighbour holds intact
/// cover — including when the cover ledger is absent (a pre-battle frame), so the smash offer
/// stays hidden until a live battle. A cell whose ledger entry is
/// [`destroyed`](gdtf_battle_sim::CoverEntry) (already smashed) is skipped — there is nothing
/// left to hit. Uses [`CoverLedger::peek`](gdtf_battle_sim::CoverLedger::peek) (the non-seeding
/// read) so only a REGISTERED (authored / already-hit) piece is offered — an unauthored empty
/// cell is not a structure. The scan order is the deterministic `(dy, dx)` Moore-8 ring.
///
/// Pure over the read cover ledger (no world mutation), so it is testable in isolation. The
/// sim's [`dispatch_melee`](gdtf_battle_sim::dispatch_melee) 8-adjacency-to-the-cell gate is the
/// authoritative re-check when the smash actually fires; this only decides what to OFFER.
pub(super) fn scan_melee_structure(actor_pos: Position, grids: &LosGrids) -> Option<CellLevel> {
    // The smash offer needs the live cover ledger; absent (pre-battle) → offer nothing.
    let cover = grids.cover.as_ref()?;

    // The actor's `(cell, level)` — x/y are the ground cell, z the storey index.
    let key = **actor_pos;
    let storey = u8::try_from(key.z).unwrap_or(0);
    let level = Level::new(storey);

    // Scan the 8 same-storey Moore-neighbour cells in a deterministic (dy, dx) order; offer the
    // first that holds an intact (not-destroyed) REGISTERED cover entry.
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue; // the actor's own cell is not adjacent.
            }
            let at = CellLevel::new(Cell::new(key.x + dx, key.y + dy), level);
            if let Some(entry) = cover.peek(&at)
                && !*entry.destroyed
            {
                return Some(at);
            }
        }
    }
    None
}

/// Scans `candidates` for the actor's actionable SHOVE target — the first 8-adjacent, ALIVE,
/// OPPOSING ganger (GTW-525).
///
/// The shove offer gate MIRRORS the sim's deliberate `dispatch_shove` gate exactly: the target
/// must be [`LifeState::Alive`] (a fresh `is_active` — NOT Downed / Dead), of the OPPOSING
/// faction, and 8-adjacent ([`is_8_adjacent`]). It is a WEAKER gate than Melee's: a shove is
/// CONTACT, so it needs NO LOS check and NO weapon — ANY ganger can shove any alive opposing
/// neighbour. Returns the first such candidate, or [`None`] when none qualifies.
///
/// Pure over the queried components (no world mutation, no grids), so it is testable in isolation.
/// The sim's [`dispatch_shove`](gdtf_battle_sim::dispatch_shove) gate is the authoritative re-check
/// when the act actually fires; this only decides what to OFFER.
pub(super) fn scan_shove_target(
    actor_pos: Position,
    actor_faction: Faction,
    candidates: &Query<CandidateReads>,
) -> Option<Entity> {
    for (entity, pos, life, faction, _stabilized, _stance) in candidates {
        // An ALIVE (NOT Downed / Dead), OPPOSING ganger within the 8-adjacent reach — the exact
        // deliberate-shove gate the sim re-checks authoritatively.
        if *life != LifeState::Alive || *faction == actor_faction || !is_8_adjacent(actor_pos, *pos)
        {
            continue;
        }
        return Some(entity);
    }
    None
}

/// Scans `doors` for the actor's actionable OPEN-DOOR target — the first 8-adjacent openable
/// terrain entity in the [`OpenState::Closed`](gdtf_battle_sim::OpenState) state (GTW-315).
///
/// A DOOR is any terrain entity carrying an [`OpenState`](gdtf_battle_sim::OpenState) (the GTW-503
/// openable mechanism attaches it only to openable pieces at spawn). The offer gate MIRRORS the
/// sim's `dispatch_open_door` gate: the door must be CLOSED (an already-open door is NOT offered —
/// the button only OPENS; closing is not a contextual act) and 8-adjacent to the actor
/// ([`is_8_adjacent`] over the actor's [`Position`] vs a [`Position`] AT the door's
/// [`TerrainCell`](gdtf_battle_sim::entity::TerrainCell) — the GTW-508 structural-melee
/// actor-vs-cell idiom the sim uses). F4 is PLAYER-ONLY, which the caller enforces by running this
/// only for a selected player-faction actor. Returns the first such door, or [`None`] when none
/// qualifies.
///
/// Pure over the queried door components (no world mutation), so it is testable in isolation. The
/// sim's [`dispatch_open_door`](gdtf_battle_sim::dispatch_open_door) gate (CLOSED `OpenState` +
/// 8-adjacent + affords the [`OpenDoorTu`](gdtf_battle_sim::tuning::OpenDoorTu) leaf) is the
/// authoritative re-check + the TU spend when the act fires (one-way `input -> sim` boundary); this
/// only decides what to OFFER.
pub(super) fn scan_open_door(actor_pos: Position, doors: &Query<DoorReads>) -> Option<Entity> {
    for (entity, open_state, door_cell) in doors {
        // Only a CLOSED door 8-adjacent to the actor is offered — the exact door gate the sim
        // re-checks authoritatively. Build a Position AT the door's cell for the actor-vs-cell
        // reach (the same idiom `dispatch_open_door` / the cover-smash scan use).
        if open_state.is_open() || !is_8_adjacent(actor_pos, Position::new(**door_cell)) {
            continue;
        }
        return Some(entity);
    }
    None
}

/// Scans `emplacements` for the actor's actionable ENTER target — the first 8-adjacent VACANT
/// weapon emplacement (GTW-543).
///
/// An EMPLACEMENT is any terrain entity carrying an
/// [`EmplacementState`](gdtf_battle_sim::EmplacementState) (the GTW-543 mechanism attaches it only
/// to emplacement pieces at spawn). The offer gate MIRRORS the sim's `dispatch_enter_emplacement`
/// gate: the emplacement must be [`EmplacementState::Vacant`](gdtf_battle_sim::EmplacementState)
/// (an occupied emplacement is NOT offered — it already has an operator) and 8-adjacent to the
/// actor ([`is_8_adjacent`] over the actor's [`Position`] vs a [`Position`] AT the emplacement's
/// [`TerrainCell`](gdtf_battle_sim::entity::TerrainCell) — the actor-vs-cell idiom the sim uses).
/// F4 is PLAYER-ONLY, which the caller enforces by running this only for a selected player-faction
/// actor. Returns the first such emplacement, or [`None`] when none qualifies.
///
/// Pure over the queried emplacement components (no world mutation), so it is testable in
/// isolation. The sim's
/// [`dispatch_enter_emplacement`](gdtf_battle_sim::acts::dispatch_enter_emplacement) gate (VACANT
/// `EmplacementState` + 8-adjacent + affords the
/// [`EnterEmplacementTu`](gdtf_battle_sim::tuning::EnterEmplacementTu) leaf) is the authoritative
/// re-check + the TU spend when the act fires (one-way `input -> sim` boundary); this only decides
/// what to OFFER.
pub(super) fn scan_enter_emplacement(
    actor_pos: Position,
    emplacements: &Query<EmplacementReads>,
) -> Option<Entity> {
    for (entity, state, emplacement_cell, _occupant) in emplacements {
        // Only a VACANT emplacement 8-adjacent to the actor is offered — the exact enter gate the
        // sim re-checks authoritatively. Build a Position AT the emplacement's cell for the
        // actor-vs-cell reach (the same idiom `dispatch_enter_emplacement` / the open-door scan use).
        if state.is_occupied() || !is_8_adjacent(actor_pos, Position::new(**emplacement_cell)) {
            continue;
        }
        return Some(entity);
    }
    None
}

/// Scans `emplacements` for the actor's actionable EXIT target — the emplacement whose recorded
/// [`EmplacementOccupant`](gdtf_battle_sim::EmplacementOccupant) IS the `actor` (GTW-543).
///
/// The offer gate MIRRORS the sim's `dispatch_exit_emplacement` gate: the emplacement's recorded
/// occupant must BE the acting selection — exit is offered ONLY to the ganger currently manning
/// it (there is NO force-eject; a ganger leaves the mount only by spending the exit TU). No
/// adjacency check is needed — the occupant is by definition on / at the mount. Returns the first
/// emplacement the actor occupies, or [`None`] when the selection is not manning any emplacement.
///
/// Pure over the queried emplacement components (no world mutation), so it is testable in
/// isolation. The sim's
/// [`dispatch_exit_emplacement`](gdtf_battle_sim::acts::dispatch_exit_emplacement) gate (the
/// recorded occupant IS the actor + affords the
/// [`ExitEmplacementTu`](gdtf_battle_sim::tuning::ExitEmplacementTu) leaf) is the authoritative
/// re-check + the TU spend when the act fires; this only decides what to OFFER.
pub(super) fn scan_exit_emplacement(
    actor: Entity,
    emplacements: &Query<EmplacementReads>,
) -> Option<Entity> {
    for (entity, _state, _cell, occupant) in emplacements {
        // Offer the emplacement whose recorded occupant IS this selection — the exact exit gate the
        // sim re-checks authoritatively (offered ONLY to the occupant, no force-eject).
        if occupant.is_some_and(|occupant| **occupant == actor) {
            return Some(entity);
        }
    }
    None
}
