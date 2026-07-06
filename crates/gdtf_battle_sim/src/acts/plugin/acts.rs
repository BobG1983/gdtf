//! Per-act dispatcher wiring — the Simulate-band act systems, each
//! `.in_set(SimSystems::Simulate)` with its explicit ordering (`bevy-traps.md` #3).

use bevy::prelude::{App, IntoScheduleConfigs, Update};

use crate::{
    acts::{
        downed::{dispatch_execute_downed, dispatch_stabilize_downed},
        enter_emplacement::{dispatch_enter_emplacement, dispatch_exit_emplacement},
        fire::dispatch_fire,
        melee::dispatch_melee,
        movement::{advance_walk, dispatch_move},
        open_door::dispatch_open_door,
        posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance},
        reload::dispatch_reload,
        shove::dispatch_shove,
        throw_grenade::dispatch_throw_grenade,
    },
    apply_pending_attachments,
    occupancy::project_path_blocking,
    occupancy_sync::{SimSystems, sync_destroyed_cover},
};

/// Wire every per-act dispatch system into the [`SimSystems::Simulate`] band with its
/// explicit ordering (`bevy-traps.md` #3) — the per-act half of
/// [`SimActsPlugin::build`](super::SimActsPlugin)'s wiring (each `add_systems` call is
/// independent; ordering is carried by `.before`/`.after`/`.in_set`, not call order).
pub(super) fn wire_acts(app: &mut App) {
    app.add_systems(
        Update,
        (
            dispatch_fire,
            dispatch_set_aiming,
            dispatch_set_stance,
            dispatch_set_facing,
            dispatch_stabilize_downed,
            dispatch_execute_downed,
            dispatch_reload,
            // GTW-507/508: the LIVE melee act. It drains MeleeRequested, gates 8-adjacency
            // (+ LOS/alive/opposing faction on the ganger arm), spends the wielded melee
            // weapon's fight-mode TU, and either runs the §7 opposed-Fight → §5 → §6
            // synthesis onto a ganger (GTW-507, REUSING the GTW-506 core + §4/§5/§6 pieces
            // verbatim) or the GTW-508 UNCONTESTED cover-smash onto an adjacent structure —
            // multiplied (mult_max) weapon damage through CoverLedger::deplete_cover, firing
            // the EXISTING CoverDestroyed signal on a lethal smash. It joins the
            // BattleInProgress-gated Simulate band (its Res grids + tuning + the three ResMut
            // RNG streams are battle-lifetime — the band's run_if skips it outside a live
            // battle, bevy-traps.md #1).
            //
            // Ordering (bevy-traps.md #3): GTW-508 promoted its `cover` param to
            // ResMut<CoverLedger> (the cover-smash arm SPENDS it), so it now DOES share
            // mutable state with sibling `dispatch_fire` (which also holds
            // ResMut<CoverLedger> for the shoot-the-cover depletion). It still joins the
            // UNORDERED group because the shared write is order-INDEPENDENT: Bevy
            // auto-serializes two ResMut on the same resource (never a data race), and the
            // only shared op — `deplete_cover`'s saturating HP decrement — commutes (two
            // decrements on the same cell reach the same remaining HP in either order) and
            // its `destroyed` flag is monotonic (set-once), so whichever dispatcher runs
            // first, the ledger and any emitted CoverDestroyed converge to the same state.
            // The presenter's swap_destroyed_cover / read_cover_destroyed react idempotently.
            // No `.before`/`.after` is needed for correctness.
            dispatch_melee,
            // GTW-315: the LIVE open-door act. It drains OpenDoorRequested, re-gates the door
            // (openable + CLOSED + 8-adjacent to the actor) and the actor (exists + affords the
            // OpenDoorTu leaf), spends that TU off the actor, and WRITES a SetOpenable::toggle for
            // the door — REUSING the GTW-503 open mechanism (it never flips OpenState directly).
            // It joins the UNORDERED group: the only state it shares with a sibling is the
            // SetOpenable message buffer it produces, which OpenableTogglePlugin's
            // apply_openable_toggle CONSUMES — but buffered messages persist a frame
            // (bevy-traps.md #4), so apply_openable_toggle reads it next tick regardless of the
            // producer/consumer intra-frame order (the door's one-frame settle, already documented
            // on the GTW-503 toggle). Its Res<CombatTuning> read is battle-lifetime — taken
            // Option<Res> so it fails closed outside a live battle (bevy-traps.md #1). Its actor
            // (&mut Tu) + door (&OpenState) queries touch disjoint entities, so no B0001 conflict.
            // No .before/.after is needed for correctness.
            dispatch_open_door,
            // GTW-543: the LIVE enter/exit-emplacement acts. They drain EnterEmplacementRequested /
            // ExitEmplacementRequested, re-gate (the emplacement is Vacant + 8-adjacent for enter,
            // or its occupant IS the actor for exit) + the actor (exists + affords the
            // Enter/Exit-EmplacementTu leaf), spend that TU off the actor, and WRITE a
            // SetEmplacement::occupy / ::vacate — REUSING the GTW-543 toggle mechanism (they never
            // flip EmplacementState directly). Like dispatch_open_door they join the UNORDERED
            // group: their only shared state is the SetEmplacement message buffer they produce,
            // which EmplacementTogglePlugin's apply_emplacement_toggle CONSUMES next tick (buffered
            // messages persist a frame, bevy-traps.md #4 — the emplacement's one-frame settle).
            // Res<CombatTuning> is Option<Res> (fail-closed, bevy-traps.md #1); their actor
            // (&mut Tu) + emplacement (&EmplacementState) queries are disjoint (no B0001).
            dispatch_enter_emplacement,
            dispatch_exit_emplacement,
        )
            .in_set(SimSystems::Simulate),
    );
    // GTW-354 (C5): the constrained move dispatch joins the same gated Simulate band
    // but is ordered `.after` the `occupancy_sync` grid-maintenance chain —
    // explicitly `.after(sync_destroyed_cover)`, its LAST system (move → die → cover)
    // — so `find_path` plans over a grid whose occupant slots have already settled
    // this frame (bevy-traps.md #3 — explicit ordering; the recompute_visibility
    // precedent). It writes `Position`/`Tu`; `sync_moved_gangers` reacts to the
    // resulting `Changed<Position>` next frame, so the two compose with no ambiguity.
    app.add_systems(
        Update,
        dispatch_move
            .after(sync_destroyed_cover)
            // GTW-501 C3: `find_path` here reads the tag-derived path-blocking
            // surface, so it must run AFTER `project_path_blocking` re-syncs it this
            // frame (a Res<OccupancyGrid> reader vs the projection's ResMut writer
            // must be ordered explicitly — bevy-traps.md #3).
            .after(project_path_blocking)
            .in_set(SimSystems::Simulate),
    );
    // GTW-355 (C6): the committed-walk engine. It advances every WalkInProgress by
    // ONE discrete step per tick — bump-stopping on the LIVE grid, charging each
    // step's planned cost atomically, halting on a reveal or a reaction interrupt.
    // Ordered `.after(dispatch_move)` so a fresh accept's WalkInProgress is visible
    // and its first step lands the same frame (the Commands sync point the ordering
    // forces makes the just-inserted component present this tick), and `.after`
    // the occupancy_sync chain (its bump-stop reads a settled grid). It writes
    // Position/Tu; recompute_visibility is ordered `.after(advance_walk)` (in
    // BattleSimPlugin) so each step's reveal is computed before the NEXT tick reads
    // it (the §44 ambush invariant; bevy-traps.md #3).
    app.add_systems(
        Update,
        advance_walk
            .after(dispatch_move)
            .after(sync_destroyed_cover)
            // GTW-501 C3/D2: the bump-stop reads the tag-derived path-blocking
            // surface (is_path_blocked) — the SAME source the planner reads — so this
            // ordering is LOAD-BEARING: it must run after the path-blocking re-sync so
            // the bump-stop sees this frame's marker changes (a `BlocksPathfinding`
            // added/removed on an in-progress walk's route is honoured the same tick),
            // keeping planner and executor in lock-step (bevy-traps.md #3).
            .after(project_path_blocking)
            .in_set(SimSystems::Simulate),
    );
    // GTW-525: the SHOVE dispatch — drains ShoveRequested (the deliberate act from the
    // input seam AND the weapon-tag auto-shove the connect hooks write) and resolves the
    // one-cell displacement (pure; the fall it may trigger routes through the shared
    // GTW-523 fork). Ordered (bevy-traps.md #3) `.after(dispatch_fire)` AND
    // `.after(dispatch_melee)` so a SAME-FRAME weapon-tag ShoveRequested — written by the
    // connecting ranged shot (dispatch_fire) or melee strike (dispatch_melee) — is in the
    // buffer when this reads it (else a one-frame lag). Its Res grids/tuning/injury +
    // ResMut fall streams are battle-lifetime, so it joins the gated Simulate band (its
    // Option<Res>/Option<ResMut> params fail closed outside a live battle, bevy-traps.md
    // #1). Param-only, no &mut World (bevy-traps.md #7).
    app.add_systems(
        Update,
        dispatch_shove
            .after(dispatch_fire)
            .after(dispatch_melee)
            .in_set(SimSystems::Simulate),
    );
    // Wire the GTW-546 throw-grenade act into the [`SimSystems::Simulate`] band — the
    // [`dispatch_throw_grenade`] dispatcher. Split out of [`wire_systems`] so that function stays
    // under clippy's line-count gate (the [`wire_clocks`] precedent).
    //
    // The dispatcher drains [`ThrowGrenadeRequested`], re-gates a blind lob (an `Arc` weapon with a
    // loaded round + affordable [`ThrowTu`](crate::tuning::ThrowTu); NO line-of-sight / facing gate),
    // marches the deterministic arc + fans the GTW-541 blast at the landing, and emits
    // [`ThrowResolved`]. It joins the same gated Simulate band as the other acts; UNORDERED — its
    // `ResMut<CoverLedger>` / `ResMut<SlabLedger>` writes commute with `dispatch_fire`'s (the
    // shared-ledger rationale documented on `dispatch_fire`), and its thrower (`&mut Tu`) + weapon
    // (`&mut Magazine`) + target queries touch disjoint entities (no B0001 conflict).
    app.add_systems(Update, dispatch_throw_grenade.in_set(SimSystems::Simulate));
    // Wire the GTW-549 weapon-attachment application system into the [`SimSystems::Simulate`]
    // band — [`apply_pending_attachments`]. Split out of [`wire_systems`] so that function stays
    // under clippy's line-count gate (the [`wire_clocks`] precedent).
    //
    // The wielded-weapon scene spawned by `setup_battle` materializes DEFERRED (on the
    // `SpawnScene` schedule), so the resolved attachment effects ride onto the weapon as a
    // [`PendingAttachments`](crate::weapon::PendingAttachments) component (composed into the
    // scene) rather than being `attach_to_weapon`'d inline (no live entity exists at setup). This
    // system runs on a later tick — once the weapon's stat components + the `PendingAttachments`
    // marker have materialized — reads the marker, and applies each effect via the mandated
    // post-spawn [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon)
    // `EntityCommand`, then removes the marker (one-shot). Ordered `.before(dispatch_fire)` so a
    // weapon's attachment stat changes are in place before any shot could read them
    // (bevy-traps.md #3; realistically the marker materializes turns before the player fires). It
    // joins the `BattleInProgress`-gated Simulate band; its Query + Commands are always valid, so
    // it needs no extra `run_if`. Param-only, no `&mut World` (bevy-traps.md #7 — the effect
    // closures' `EntityWorldMut` access is the ticket's sanctioned carve-out).
    app.add_systems(
        Update,
        apply_pending_attachments
            .before(dispatch_fire)
            .in_set(SimSystems::Simulate),
    );
}
