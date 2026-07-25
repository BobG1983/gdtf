//! Turn-cycle, AI, and per-round clock wiring — the turn engine, the enemy brain,
//! the bleed/DOT/field clocks, the injury applier, and the on-death resolver.

use bevy::prelude::{App, IntoScheduleConfigs, Update, resource_exists};

use crate::{
    acts::{
        downed::dispatch_stabilize_downed, fire::dispatch_fire, injury::apply_injury,
        melee::dispatch_melee, movement::dispatch_move,
    },
    ai::enemy_ai_turn,
    effects::{
        bleed::{enemy_phase_started, mark_downed_bleeding, tick_bleed},
        dot::{apply_dot, tick_dot},
        fields::tick_fields,
        on_death::resolve_on_death,
    },
    falls::apply_falls,
    occupancy::project_path_blocking,
    occupancy_sync::SimSystems,
    turn::{ActiveFaction, dispatch_end_turn},
};

/// Wire the turn-cycle engine, the enemy-AI brain, the per-round clocks, and the
/// injury applier into the [`SimSystems::Simulate`] band — the turn/clock half of
/// [`SimActsPlugin::build`](super::SimActsPlugin)'s wiring (each `add_systems` call is
/// independent; ordering is carried by `.before`/`.after`/`.run_if`, not call order).
pub(super) fn wire_turn_clocks(app: &mut App) {
    // GTW-309: the turn-cycle engine joins the gated Simulate band, but takes the
    // battle-lifetime ActiveFaction resource (co-inserted with BattleInProgress),
    // so it carries its OWN resource_exists::<ActiveFaction> run_if to keep its
    // ResMut<ActiveFaction> read panic-free (bevy-traps.md #1).
    app.add_systems(
        Update,
        dispatch_end_turn
            .run_if(resource_exists::<ActiveFaction>)
            .in_set(SimSystems::Simulate),
    );
    // GTW-70: the minimal enemy-AI brain. It joins the gated Simulate band and is
    // ordered `.after(dispatch_end_turn)` (so it sees the freshly-handed-off
    // ActiveFaction + the enemy team's regenerated TU this frame) and
    // `.before(dispatch_fire)` / `.before(dispatch_move)` (so the REAL FireRequested
    // / MoveRequested it emits dispatch the SAME frame — and the pre-checked accept
    // guarantee holds against the same-frame world). It carries its OWN
    // `run_if(resource_exists::<ActiveFaction>)` so its read stays panic-free
    // outside a live battle (bevy-traps.md #1), and reads no SquadVisibility (it
    // plans on the immutable OmniscientFog), so it adds no ordering ambiguity with
    // the recompute_visibility writer.
    app.add_systems(
        Update,
        enemy_ai_turn
            .run_if(resource_exists::<ActiveFaction>)
            .after(dispatch_end_turn)
            // GTW-501 C3: `reachable_within` here reads the tag-derived path-blocking
            // surface, so it must run AFTER `project_path_blocking` re-syncs it this
            // frame (read-after-write ordering on OccupancyGrid — bevy-traps.md #3).
            .after(project_path_blocking)
            .before(dispatch_fire)
            .before(dispatch_move)
            .in_set(SimSystems::Simulate),
    );
    // GTW-336: wire the §9 bleed-out clock into the live runtime. `tick_bleed`
    // drains a flat tuning BleedRate of Wounds from every un-stabilized Downed
    // ganger ONCE PER FULL ROUND, AT THE ENEMY-PHASE START (resolution.md §9). The
    // turn-cycle engine `dispatch_end_turn` emits a TurnStarted at each advance and
    // the enemy one fires exactly once per full round, so `tick_bleed` runs
    // `.after(dispatch_end_turn)` (so the frame's TurnStarted is buffered) and
    // `.run_if(enemy_phase_started)` (true iff a non-player TurnStarted was emitted
    // this frame — its own independent reader, so it never steals the boundary from
    // the combat-log reader). It joins the BattleInProgress-gated Simulate band, so
    // its Res<CombatTuning> read is panic-free outside a live battle (the band's
    // run_if skips it — bevy-traps.md #1).
    //
    // GTW-641 (the turn-start ruling): on the boundary frame the tick is ALSO pinned
    // `.before` the act dispatchers (fire / move / melee) — the new turn's acts
    // resolve the SAME frame the enemy TurnStarted lands (the AI's volley dispatches
    // this frame), and without the pin tick_bleed vs dispatch_fire is unordered
    // (bevy-traps.md #3): a ganger THAT volley downs could nondeterministically
    // drain a Wound the instant it fell (down zero rounds — the exact GTW-641
    // off-by-one, cross-system). "Bleeds happen at turn start" = the clock resolves
    // AT the boundary, before the new turn's act resolution.
    app.add_systems(
        Update,
        tick_bleed
            .after(dispatch_end_turn)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    );
    // GTW-695: reify the apply_hit down-gate as the §9 BleedingOut condition. The damage
    // pipeline (fire / melee / falls → apply_hit) writes LifeState::Downed in place with
    // NO Commands (bevy-traps.md #7 — the whole pipeline mutates via queries/messages), so
    // this change-detection system inserts the BleedingOut marker on each ganger the damage
    // pipeline just downed (the OTHER down-transition site, tick_bleed's own injury-HP
    // bleed, inserts inline). Ordered `.after` every damage dispatcher (so this frame's
    // Downed writes are visible) AND `.after(tick_bleed)` with a `Without<BleedingOut>`
    // filter, so a ganger tick_bleed downed + marked inline this frame is not double-handled
    // (bevy-traps.md #3). It writes only deferred Commands, so the marker is visible to the
    // NEXT tick_bleed round — a damage-downed ganger bleeds from the following enemy phase,
    // exactly as the flag model's LifeState::Downed read did. Its Query + Commands are always
    // valid, so it joins the gated Simulate band with no extra run_if.
    app.add_systems(
        Update,
        mark_downed_bleeding
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(apply_falls)
            .after(tick_bleed)
            // Ordered BEFORE the stabilize dispatcher (bevy-traps.md #3): if a ganger is
            // both downed and stabilized in the SAME frame, stabilize's marker REMOVAL must
            // win — so this insertion runs first and stabilize removes it after (never the
            // reverse, which would re-insert the just-removed condition on a still-Changed
            // LifeState). In normal play the two never coincide (stabilize is turns after the
            // down), but the ordering makes the coincident case deterministic.
            .before(dispatch_stabilize_downed)
            .in_set(SimSystems::Simulate),
    );
    // GTW-438: the injury applier — drains the InjuryInflicted buffer and folds
    // each rolled injury into its target's InflictedInjuries (via `gain`, tripping
    // Changed) + syncs the standalone BleedAfflicted component. Ordered `.after`
    // EVERY InjuryInflicted producer (bevy-traps.md #3 — explicit ordering, never
    // registration order): the ranged fire act (dispatch_fire), the GTW-821 melee
    // act (dispatch_melee) and the fall path (apply_falls, wired in the sibling
    // FallsPlugin — the ordering is a no-op if that plugin is absent). So an injury
    // rolled THIS frame, from whichever source, is applied THIS tick — the resulting
    // Changed<InflictedInjuries> is then projected by
    // `rederive_stats_on_injury_change`, which is ordered
    // `.after(SimSystems::Simulate)` in BattleSimPlugin (so the ledger gain →
    // projector re-derive settles within the frame, before the next tick's stat
    // reads; bevy-traps.md #3 / #7 — query/Commands/MessageReader, no &mut World).
    // It joins the gated Simulate band (no resource it reads is battle-lifetime —
    // the Query + Commands are always valid — so it needs no extra run_if).
    app.add_systems(
        Update,
        apply_injury
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(apply_falls)
            .in_set(SimSystems::Simulate),
    );
    wire_clocks(app); // GTW-544/547: the DOT+field clocks + on-death resolver (line-cap split)
}

/// Wire the per-round-clock + on-death systems into the [`SimSystems::Simulate`] band — the
/// GTW-544 [`tick_dot`] / GTW-545 [`tick_fields`] clocks + applier and the GTW-547
/// [`resolve_on_death`] resolver. One combined split out of [`wire_turn_clocks`] so that function
/// stays under clippy's line-count gate (the `register_messages` / `wire_systems` split
/// precedent).
fn wire_clocks(app: &mut App) {
    wire_dot(app);
    wire_on_death(app);
}

/// Wire the GTW-547 on-death-effect resolver into the [`SimSystems::Simulate`] band — the
/// [`resolve_on_death`] applier. Split out of [`wire_clocks`] so that helper stays focused.
fn wire_on_death(app: &mut App) {
    app.add_systems(
        Update,
        // GTW-547: `resolve_on_death` drains the OnDeathOccurred buffer and fans each dying
        // source's authored on-death effect (Explode fans a GTW-541 AoE blast; LeaveField spawns
        // a GTW-545 field), resolving cascading explosions to a same-frame fixpoint. Ordered
        // (bevy-traps.md #3) `.after` EVERY death producer so THIS frame's deaths are all
        // buffered before it reads: the ranged/melee kills (dispatch_fire / dispatch_melee) + the
        // per-round bleed/DOT/field clocks (tick_bleed / tick_dot / tick_fields) + the falls kill
        // (apply_falls, wired in the sibling FallsPlugin — the ordering is a no-op if that plugin
        // is absent, and messages persist a frame regardless). It joins the BattleInProgress-gated
        // Simulate band; its Res<OccupancyGrid> / ResMut<FieldRegistry> / Res<CoverOnDeathRegistry>
        // reads are sim-`setup_battle`-inserted battle-lifetime resources, so it carries explicit
        // `resource_exists` run-ifs to stay panic-free if the band runs without them. The field
        // CATALOG (FieldDefRegistry) is app/Load-owned (NOT sim-set), so it is taken Option<Res>
        // INSIDE the system rather than gated on (bevy-traps.md #1).
        resolve_on_death
            .after(dispatch_fire)
            .after(dispatch_melee)
            .after(tick_bleed)
            .after(tick_dot)
            .after(tick_fields)
            .after(apply_falls)
            .run_if(resource_exists::<crate::occupancy::OccupancyGrid>)
            .run_if(resource_exists::<crate::effects::fields::FieldRegistry>)
            .run_if(resource_exists::<crate::effects::on_death::CoverOnDeathRegistry>)
            .in_set(SimSystems::Simulate),
    );
}

/// Wire the GTW-544 damage-over-time systems into the [`SimSystems::Simulate`] band — the
/// [`apply_dot`] boundary applier + the per-round [`tick_dot`] clock. Split out of
/// [`wire_turn_clocks`] so that function stays under clippy's line-count gate.
fn wire_dot(app: &mut App) {
    app.add_systems(
        Update,
        // GTW-544: the DOT applier — drains the DotApplied buffer and attaches (or REFRESHES,
        // refresh-not-stack) a Dot on each struck ganger. Ordered `.after(dispatch_fire)` so a
        // SAME-FRAME penetrating DOT hit (the fire act emitted it this frame) is applied this
        // tick (the `apply_injury` precedent). No resource it reads is battle-lifetime (the
        // Query + Commands + MessageReader are always valid), so it needs no extra run_if.
        // Param-only, no &mut World (bevy-traps.md #7).
        apply_dot.after(dispatch_fire).in_set(SimSystems::Simulate),
    )
    // GTW-544: wire the DOT clock into the live runtime — the EXACT `tick_bleed` cadence.
    // `tick_dot` drains each afflicted ganger's Hp DIRECTLY by its Dot's per-turn damage (no
    // armor matchup / injury roll / RNG) ONCE PER FULL ROUND, AT THE ENEMY-PHASE START (the
    // SAME `enemy_phase_started` run condition the §9 bleed-out clock uses, so both fire once
    // per full round). Ordered `.after(dispatch_end_turn)` (so the frame's TurnStarted is
    // buffered) and `.run_if(enemy_phase_started)` (its own independent reader, so it never
    // steals the boundary from the combat-log / bleed readers). It joins the
    // BattleInProgress-gated Simulate band, so its Query is exercised only in a live battle
    // (bevy-traps.md #1).
    //
    // GTW-658 (the turn-start ruling, the GTW-641 tick_bleed precedent): on the boundary
    // frame the tick is ALSO pinned `.before` the act dispatchers (fire / move / melee) —
    // the new turn's acts resolve the SAME frame the enemy TurnStarted lands (the AI's
    // volley dispatches this frame), and without the pin tick_dot vs the dispatchers is
    // unordered (bevy-traps.md #3): the DOT's Hp drain and a same-frame act's resolution
    // could interleave nondeterministically (the same hazard class GTW-641 fixed for
    // tick_bleed — Hp clocks here, no Wounds semantics). The clocks resolve AT the
    // boundary, before the new turn's act resolution.
    .add_systems(
        Update,
        tick_dot
            .after(dispatch_end_turn)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    )
    // GTW-545: wire the area-damage-field clock into the live runtime — the EXACT `tick_dot` /
    // `tick_bleed` cadence. `tick_fields` reads each fielded cell's occupant off the
    // OccupancyGrid and drains its Hp DIRECTLY by the field's per-turn damage (no armor
    // matchup / injury roll / RNG; gated ONLY by whole-source armor-type immunity) ONCE PER
    // FULL ROUND, AT THE ENEMY-PHASE START (the SAME `enemy_phase_started` run condition), then
    // counts every Turns field down + removes the expired ones. Ordered `.after(dispatch_end_turn)`
    // (so the frame's TurnStarted is buffered) and `.run_if(enemy_phase_started)` (its own
    // independent reader, so it never steals the boundary from the combat-log / bleed / DOT
    // readers), AND explicitly `.after(tick_dot)` (bevy-traps.md #3): both drains share the
    // enemy-phase cadence and can empty one occupant's Hp the same frame, so pinning tick_fields
    // after tick_dot makes a two-source lethal frame's Dead-flip + emit order deterministic
    // (the Dead-flip is idempotent, but the order is pinned). It joins the BattleInProgress-gated
    // Simulate band, so its Res<OccupancyGrid> / ResMut<FieldRegistry> reads are exercised only
    // in a live battle (bevy-traps.md #1).
    //
    // GTW-658 (the turn-start ruling, the GTW-641 tick_bleed precedent): on the boundary
    // frame the tick is ALSO pinned `.before` the act dispatchers (fire / move / melee) —
    // the new turn's acts resolve the SAME frame the enemy TurnStarted lands (the AI's
    // volley dispatches this frame), and without the pin tick_fields vs the dispatchers is
    // unordered (bevy-traps.md #3): a fielded ganger's field drain and a same-frame act's
    // resolution could interleave nondeterministically (the same hazard class GTW-641 fixed
    // for tick_bleed — Hp clocks here, no Wounds semantics). The clocks resolve AT the
    // boundary, before the new turn's act resolution.
    .add_systems(
        Update,
        tick_fields
            .after(dispatch_end_turn)
            .after(tick_dot)
            .before(dispatch_fire)
            .before(dispatch_move)
            .before(dispatch_melee)
            .run_if(enemy_phase_started)
            .run_if(resource_exists::<crate::effects::fields::FieldRegistry>)
            .in_set(SimSystems::Simulate),
    );
}
