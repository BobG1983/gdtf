//! The **enemy-turn brain** [`enemy_ai_turn`] — the frame-paced minimal AI that, on the
//! enemy faction's turn, makes each enemy ganger engage-or-advance-or-hold and ends the
//! turn back to the player when the enemy is done (GTW-70 §A / §B / §D).
//!
//! It REUSES the existing combat surfaces verbatim — no combat is re-implemented:
//!
//! - **engage** emits the REAL [`FireRequested`] the landed
//!   [`dispatch_fire`](crate::acts::dispatch_fire) resolves, gated by the SHARED
//!   [`can_see`](crate::los::can_see) (one LOS truth) + [`can_fire`](crate::magazine::can_fire) (the shared fire guard) + [`can_engage`](crate::acts::can_engage)
//!   (the SHARED arc verdict, GTW-70 leaf 2);
//! - **advance** emits the REAL [`MoveRequested`] the landed
//!   [`dispatch_move`](crate::acts::dispatch_move) resolves, planning over the SAME
//!   [`OmniscientFog`] move fog the executor's `move_fog` selects for a non-player mover
//!   (GTW-70 leaf 3);
//! - **hold / done** emits nothing, then ends the turn with a real [`EndTurnRequested`].
//!
//! Param-only — `Query` / `Res` / [`MessageWriter`] — NO `&mut World` (`bevy-traps.md` #7).
//! Deterministic-without-RNG: every choice is a pure function of game state ordered by the
//! `(level, y, x)` total order, and gangers are visited in that same order so the
//! downstream `ShotRng` / `SeverityRng` draws (inside `fire()`) consume in a reproducible
//! order — replay-stable (GTW-70 §E).

use bevy::prelude::{Entity, MessageWriter, Res};

use super::{
    advance::plan_reposition,
    decide::{AiTarget, pick_nearest},
    engage::{WeaponLookup, engageable_targets},
    snapshot::{EnemyTurnGangers, GangerRow, MidWalk, cell_order},
};
use crate::{
    acts::{EndTurnRequested, FireRequested, MoveRequested},
    battle::PlayerFaction,
    cover::CoverLedger,
    ganger::LifeState,
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    magazine::{Magazine, mode_tu_cost},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::OmniscientFog,
    weapon::Handedness,
};

/// The **enemy-turn brain** — on the enemy faction's turn, run one engage-or-advance-or-hold
/// pass over each enemy ganger and end the turn back to the player when the enemy is done
/// (GTW-70 §A / §B / §C / §D).
///
/// Per FRAME (the brain is stateless across frames — it re-derives everything from current
/// world state):
///
/// 1. If [`ActiveFaction`] is the player's, return (the human's turn).
/// 2. Snapshot every ganger as a Copy `GangerRow`; the acting enemies (`faction ==
///    active`) are visited in `(level, y, x)` order, the opposing gangers (`faction !=
///    active`) are the FIRE targets + advance goals.
/// 3. For each conscious, not-mid-walk enemy, in order — STOPPING at the FIRST that acts,
///    so a long enemy turn emits its acts across ticks rather than dumping a whole turn's
///    worth into one frame:
///    - **ENGAGE** (§B clause 1 + §C): resolve its weapon (the [`Magazine`] + single-shot
///      [`FireModeSpec`](crate::weapon::FireModeSpec) off the related weapon entity, exactly
///      as `dispatch_fire` does); a target is engageable iff [`can_see`](crate::los::can_see) (real per-pair LOS)
///      ∧ [`can_fire`](crate::magazine::can_fire) (the shared fire guard) ∧ [`can_engage`](crate::acts::can_engage) (the SHARED `¬Reject` arc
///      verdict). Pick the nearest engageable ([`pick_nearest`]) and emit the REAL
///      [`FireRequested`] (single mode); that's the enemy's ONE act this frame.
///    - **ADVANCE** (§D.2): no engageable target → plan a reposition toward the nearest
///      opposing ganger's actual cell over the [`OmniscientFog`] move fog
///      ([`reachable_within`](crate::pathfinder::reachable_within) + [`plan_advance`](super::decide::plan_advance)) and emit the REAL [`MoveRequested`].
///    - **HOLD**: emit nothing.
/// 4. End-turn, decoupled from emission (§D.3): emit [`EndTurnRequested`] ONLY when no enemy
///    acted this frame AND none is mid-walk — so the turn always terminates (the fire path's
///    `can_engage` pre-check and the move path's shared-fog `reachable_within` make every
///    emitted act dispatcher-accepted, so each act spends TU or attaches a walk, the
///    strictly-decreasing termination measure).
///
/// Firing uses the REAL per-pair [`can_see`](crate::los::can_see) (the AI never gets to shoot through the
/// omniscient move fog — that fog is the MOVE planner's only); the symmetric enemy
/// fog-of-war is deferred to GTW-71 (GTW-70 §D.1 / §F).
#[expect(
    clippy::too_many_arguments,
    reason = "the enemy brain reads the active/player factions, the combat tuning, the five \
              read grids (occupancy / surface / cover / links / floor costs) + the AI move \
              fog, the ganger + wielded-weapon + \
              weapon-entity queries, and the three act MessageWriters; each is a distinct \
              Bevy SystemParam, mirroring dispatch_fire's own argument-count carve-out, and \
              bundling them would only hide the real reads"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the brain is ONE cohesive per-frame engage-or-advance-or-hold pass over the \
              enemy gangers (snapshot → sorted decision loop → emission-decoupled turn-end); \
              splitting it would thread the grids/tuning/snapshot + the two borrowed closures \
              (is_dead / relation_of) through helpers, obscuring the access set more than the \
              length costs (the setup_battle too_many_lines precedent)"
)]
pub fn enemy_ai_turn(
    active: Res<ActiveFaction>,
    player: Option<Res<PlayerFaction>>,
    tuning: Res<CombatTuning>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    links: Res<VerticalLinkGraph>,
    floor_costs: Res<FloorCostGrid>,
    omniscient: Option<Res<OmniscientFog>>,
    gangers: EnemyTurnGangers,
    // GTW-505: the weapon-resolution bundle (wields + weapon-stat query + the melee-marker
    // probe), grouped so the brain stays under Bevy's 16-param SystemParam-tuple arity.
    weapon_lookup: WeaponLookup,
    mut fire_writer: MessageWriter<FireRequested>,
    mut move_writer: MessageWriter<MoveRequested>,
    mut end_turn_writer: MessageWriter<EndTurnRequested>,
) {
    // Battle-lifetime guard: without a player faction there is no live battle to drive
    // (bevy-traps #1). The brain acts ONLY on the enemy faction's turn.
    let Some(player_faction) = player.as_deref().map(|player| **player) else {
        return;
    };
    let active_faction = **active;
    if active_faction == player_faction {
        return;
    }

    // Snapshot every ganger as a Copy row, so the closures below borrow the snapshot (not
    // the live query) and every ordering is a pure function of game state.
    let rows: Vec<GangerRow> = gangers
        .iter()
        .map(
            |(
                entity,
                position,
                stance,
                facing,
                aiming,
                life,
                tu,
                tu_max,
                faction,
                walking,
                injuries,
            )| {
                GangerRow {
                    entity,
                    position: *position,
                    stance: *stance,
                    facing: *facing,
                    aiming: *aiming,
                    life: *life,
                    tu: *tu,
                    tu_max: *tu_max,
                    faction: *faction,
                    walking: MidWalk::new(walking),
                    // GTW-443: fold the available hand count from the ledger now (an
                    // absent ledger = the uninjured two-hands default).
                    hands: injuries
                        .map_or_else(HandsAvailable::default, InflictedInjuries::hands_available),
                    // GTW-444: fold the movement-cost factor from the ledger now (an absent
                    // ledger = IDENTITY, 1.0 — no slowdown).
                    factor: injuries.map_or(
                        MovementCostFactor::IDENTITY,
                        InflictedInjuries::movement_cost_factor,
                    ),
                }
            },
        )
        .collect();

    // The acting enemies (faction == active), in (level, y, x) order (deterministic — never
    // raw query order, bevy-traps #3).
    let mut enemies: Vec<GangerRow> = rows
        .iter()
        .copied()
        .filter(|row| row.faction == active_faction)
        .collect();
    enemies.sort_by_key(|row| cell_order(&row.position));

    // Every opposing ganger (faction != active) — the FIRE targets and advance goals.
    let targets: Vec<GangerRow> = rows
        .iter()
        .copied()
        .filter(|row| row.faction != active_faction)
        .collect();
    let all_targets: Vec<AiTarget> = targets
        .iter()
        .map(|row| {
            // The canonical CellLevel accessors through Position's deref (GTW-565).
            AiTarget::new(row.entity, row.position.cell(), row.position.level())
        })
        .collect();

    // A Dead ganger is a corpse the LOS march flies THROUGH (GTW-317) — the same is_dead
    // pass-through can_see / has_los take. An entity absent from the snapshot is no corpse.
    // Bound to a Copy reference so it can be handed to `can_see` once per target across the
    // inner loop without moving the closure (the recompute_visibility is_dead precedent).
    let is_dead_fn = |entity: Entity| {
        rows.iter()
            .find(|row| row.entity == entity)
            .is_some_and(|row| row.life == LifeState::Dead)
    };
    let is_dead = &is_dead_fn;
    let mut acted = false;
    for enemy in &enemies {
        // Skip a Downed/Dead enemy (cannot act) and one mid-walk (its one act resolves
        // across frames — it stays "busy" until advance_walk finishes; GTW-70 §D.3).
        if !*enemy.life.is_active() || *enemy.walking {
            continue;
        }

        let enemy_cell = enemy.position.cell();
        let enemy_level = enemy.position.level();

        // (1) ENGAGE — resolve the enemy's weapon (Magazine + single-shot FireModeSpec +
        //     Handedness off the related weapon entity, exactly as dispatch_fire reads it),
        //     then engage iff some opposing ganger passes the SHARED gate: can_see ∧ can_fire
        //     (incl. the GTW-443 hand-count clause folded from the enemy's injury ledger) ∧
        //     ¬Reject arc.
        // GTW-505 C5 / GTW-543 / GTW-673: resolve the weapon dispatch would FIRE through the
        // shared preference rule (`Wields::firing_weapon`, mounted-first, melee-excluded), so
        // the brain engage-gates on the mount a manning enemy would fire — never its carried
        // gun — and never the melee weapon.
        let weapon_data = weapon_lookup.firing(enemy.entity);
        if let Some((magazine, fire_mode, handedness)) = weapon_data {
            let mode = fire_mode.single();
            let magazine: Magazine = *magazine;
            let handedness: Handedness = *handedness;
            let fire_cost = mode_tu_cost(&mode, &enemy.tu_max, &enemy.aiming, &tuning);
            let engageable = engageable_targets(
                enemy,
                &targets,
                (magazine, mode, handedness),
                fire_cost,
                &occupancy,
                &surface,
                &cover,
                &tuning,
                is_dead,
            );
            if let Some(target) = pick_nearest(enemy_cell, enemy_level, &engageable) {
                fire_writer.write(FireRequested::new(
                    enemy.entity,
                    mode,
                    target.cell,
                    target.level,
                ));
                acted = true;
                // ONE act per TICK — stop at the first enemy to act. This is emission
                // SHAPE, not a wait: the brain now runs at full speed (~60 passes a
                // second), and bounding each pass to one act is what keeps a long enemy
                // turn from dumping its whole act list into a single frame. GTW-727 retired
                // the tick-count DWELL that used to sit on top of this; the bound itself
                // stays.
                break;
            }
        }

        // (2) ADVANCE — no engageable target: step toward the nearest opposing ganger's
        //     actual cell over the OmniscientFog move fog (see plan_reposition's §D.1/§D.2
        //     doc). Absent the fog (no live battle) the brain can't plan and HOLDs.
        if let (Some(goal), Some(omniscient)) = (
            pick_nearest(enemy_cell, enemy_level, &all_targets),
            omniscient.as_deref(),
        ) && let Some(dest) = plan_reposition(
            enemy,
            &goal,
            &rows,
            omniscient,
            &occupancy,
            &links,
            &tuning,
            &floor_costs,
        ) {
            move_writer.write(MoveRequested::new(enemy.entity, dest));
            acted = true;
            // ONE act per TICK — see the fire arm above.
            break;
        }
        // (3) HOLD — try the NEXT enemy this same pass (a held enemy emits nothing).
    }

    // Turn-end, decoupled from emission (§D.3): end the turn ONLY when no enemy acted this
    // frame AND none is mid-walk (a WalkInProgress attached on an earlier frame). `acted`
    // is set only behind a dispatcher-guaranteed-accept pre-check, so an emitted act always
    // spends TU (fire) or attaches a walk (move) — the brain then waits rather than
    // re-emitting, and EndTurnRequested fires only when the enemy genuinely has nothing left.
    let any_walking = enemies.iter().any(|enemy| *enemy.walking);
    if !acted && !any_walking {
        end_turn_writer.write(EndTurnRequested);
    }
}
