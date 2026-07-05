//! Turn-cycle, AI, and per-round clock wiring — the turn engine, the enemy brain,
//! the bleed/DOT/field clocks, the injury applier, and the on-death resolver.

use bevy::prelude::{App, IntoScheduleConfigs, Update, resource_exists};

use crate::{
    acts::{
        fire::dispatch_fire, injury::apply_injury, melee::dispatch_melee, movement::dispatch_move,
    },
    ai::enemy_ai_turn,
    bleed::{enemy_phase_started, tick_bleed},
    dot::{apply_dot, tick_dot},
    falls::apply_falls,
    fields::tick_fields,
    occupancy::project_path_blocking,
    occupancy_sync::SimSystems,
    on_death::resolve_on_death,
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
    app.add_systems(
        Update,
        tick_bleed
            .after(dispatch_end_turn)
            .run_if(enemy_phase_started)
            .in_set(SimSystems::Simulate),
    );
    // GTW-438: the injury applier — drains the InjuryInflicted buffer and folds
    // each rolled injury into its target's InflictedInjuries (via `gain`, tripping
    // Changed) + syncs the standalone BleedAfflicted component. Ordered
    // `.after(dispatch_fire)` so a SAME-FRAME injury (the fire act emitted it this
    // frame) is applied this tick — the resulting Changed<InflictedInjuries> is
    // then projected by `rederive_stats_on_injury_change`, which is ordered
    // `.after(SimSystems::Simulate)` in BattleSimPlugin (so the ledger gain →
    // projector re-derive settles within the frame, before the next tick's stat
    // reads; bevy-traps.md #3 / #7 — query/Commands/MessageReader, no &mut World).
    // It joins the gated Simulate band (no resource it reads is battle-lifetime —
    // the Query + Commands are always valid — so it needs no extra run_if).
    app.add_systems(
        Update,
        apply_injury
            .after(dispatch_fire)
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
            .run_if(resource_exists::<crate::fields::FieldRegistry>)
            .run_if(resource_exists::<crate::on_death::CoverOnDeathRegistry>)
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
    .add_systems(
        Update,
        tick_dot
            .after(dispatch_end_turn)
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
    .add_systems(
        Update,
        tick_fields
            .after(dispatch_end_turn)
            .after(tick_dot)
            .run_if(enemy_phase_started)
            .run_if(resource_exists::<crate::fields::FieldRegistry>)
            .in_set(SimSystems::Simulate),
    );
}
