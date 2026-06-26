//! The **enemy-turn brain** [`enemy_ai_turn`] — the frame-paced minimal AI that, on the
//! enemy faction's turn, makes each enemy ganger engage-or-advance-or-hold and ends the
//! turn back to the player when the enemy is done (GTW-70 §A / §B / §D).
//!
//! It REUSES the existing combat surfaces verbatim — no combat is re-implemented:
//!
//! - **engage** emits the REAL [`FireRequested`] the landed
//!   [`dispatch_fire`](crate::acts::dispatch_fire) resolves, gated by the SHARED
//!   [`can_see`] (one LOS truth) + [`can_fire`] (the shared fire guard) + [`can_engage`]
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

use bevy::{
    ecs::query::Has,
    prelude::{Entity, MessageWriter, Query, Res},
};

use super::decide::{AiTarget, pick_nearest, plan_advance};
use crate::{
    acts::{EndTurnRequested, FireRequested, MoveRequested, can_engage},
    battle::PlayerFaction,
    cover::CoverLedger,
    fire::WieldsQuery,
    ganger::{Aiming, Facing, Faction, LifeState, Position, Stance, Tu, TuMax},
    injuries::{HandsAvailable, InflictedInjuries, MovementCostFactor},
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    move_acts::WalkInProgress,
    occupancy::OccupancyGrid,
    pathfinder::{PlanningView, reachable_within},
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog},
    weapon::{FireMode, Handedness, Wields},
};

/// The brain's read-only ganger snapshot query shape — every ganger's brain-relevant
/// components plus its mid-walk flag, read out into a Copy [`GangerRow`] each frame
/// (factored into a `type` so the system signature stays under clippy's type-complexity
/// gate, the [`ShooterQuery`](crate::fire::ShooterQuery) precedent).
type EnemyTurnGangers<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static Position,
        &'static Stance,
        &'static Facing,
        &'static Aiming,
        &'static LifeState,
        &'static Tu,
        &'static TuMax,
        &'static Faction,
        Has<WalkInProgress>,
        // GTW-443: the enemy's injury ledger, read OPTIONALLY (an absent ledger = the
        // uninjured two-hands default), folded into the row's `hands` for the SHARED
        // can_fire hand-count gate — so the AI is refused a TwoHanded weapon below two
        // hands exactly as the player path is.
        Option<&'static InflictedInjuries>,
    ),
>;

/// A Copy snapshot of one ganger's brain-relevant state, read out of the live query so the
/// decision pass orders + closes over plain data — never raw `Query` iteration order
/// (`bevy-traps.md` #3 / GTW-70 §E).
#[derive(Clone, Copy)]
struct GangerRow {
    /// The ganger entity — the act emissions' actor/target ref.
    entity:   Entity,
    /// Its `(cell, level)` grid position.
    position: Position,
    /// Its stance — the eye / silhouette anchor for `can_see`.
    stance:   Stance,
    /// Its facing — the arc datum for `can_engage`.
    facing:   Facing,
    /// Its aim mode — the per-shot TU premium selector.
    aiming:   Aiming,
    /// Its life state — only an active (Alive) enemy acts; only an active observer sees.
    life:     LifeState,
    /// Its current TU pool — the fire / move affordability budget.
    tu:       Tu,
    /// Its round-start TU ceiling — the denominator of the per-shot TU charge.
    tu_max:   TuMax,
    /// Its gang — splits acting enemies (`== active`) from targets (`!= active`).
    faction:  Faction,
    /// Whether it is mid-walk (`Has<WalkInProgress>`) — skipped while busy, but it keeps
    /// the turn open (the §D.3 `busy` measure).
    walking:  bool,
    /// Its available hand count (GTW-443) — folded from its injury ledger at snapshot
    /// time (an absent ledger = the uninjured two-hands default), fed to the SHARED
    /// `can_fire` hand-count gate.
    hands:    HandsAvailable,
    /// Its movement-cost factor (GTW-444) — the "Hampered" slowdown folded from its injury
    /// ledger at snapshot time (an absent ledger = [`MovementCostFactor::IDENTITY`], `1.0`),
    /// fed to [`reachable_within`] so a Hampered enemy's advance plan respects the SAME
    /// per-step slowdown the move dispatch will charge it.
    factor:   MovementCostFactor,
}

/// The ground cell `(x, y)` of a [`Position`] — the z storey dropped (the `fire.rs`
/// `actor_cell` split precedent).
fn row_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}

/// The storey [`Level`] of a [`Position`].
fn row_level(position: &Position) -> Level {
    let key = ***position;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is a storey index in 0..MAX_LEVELS (8) by construction, so the i32 -> \
                  u8 narrowing cannot truncate or sign-flip"
    )]
    let storey = key.z as u8;
    Level::new(storey)
}

/// The `(cell, level)` key of a [`Position`] — one [`Position`]→[`CellLevel`] deref.
fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// The `(level, y, x)` sort key of a [`Position`] — the deterministic total order the brain
/// visits actors in (`bevy-traps.md` #3 / GTW-70 §E).
fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

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
/// 3. For each conscious, not-mid-walk enemy, in order:
///    - **ENGAGE** (§B clause 1 + §C): resolve its weapon (the [`Magazine`] + single-shot
///      [`FireModeSpec`](crate::weapon::FireModeSpec) off the related weapon entity, exactly
///      as `dispatch_fire` does); a target is engageable iff [`can_see`] (real per-pair LOS)
///      ∧ [`can_fire`] (the shared fire guard) ∧ [`can_engage`] (the SHARED `¬Reject` arc
///      verdict). Pick the nearest engageable ([`pick_nearest`]) and emit the REAL
///      [`FireRequested`] (single mode); that's the enemy's ONE act this frame.
///    - **ADVANCE** (§D.2): no engageable target → plan a reposition toward the nearest
///      opposing ganger's actual cell over the [`OmniscientFog`] move fog
///      ([`reachable_within`] + [`plan_advance`]) and emit the REAL [`MoveRequested`].
///    - **HOLD**: emit nothing.
/// 4. End-turn, decoupled from emission (§D.3): emit [`EndTurnRequested`] ONLY when no enemy
///    acted this frame AND none is mid-walk — so the turn always terminates (the fire path's
///    `can_engage` pre-check and the move path's shared-fog `reachable_within` make every
///    emitted act dispatcher-accepted, so each act spends TU or attaches a walk, the
///    strictly-decreasing termination measure).
///
/// Firing uses the REAL per-pair [`can_see`] (the AI never gets to shoot through the
/// omniscient move fog — that fog is the MOVE planner's only); the symmetric enemy
/// fog-of-war is deferred to GTW-71 (GTW-70 §D.1 / §F).
#[expect(
    clippy::too_many_arguments,
    reason = "the enemy brain reads the active/player factions, the combat tuning, the five \
              read grids (occupancy / surface / cover / links / floor costs) + the AI move \
              fog, the ganger + wielded-weapon + weapon-entity queries, and the three act \
              MessageWriters; each is a distinct Bevy SystemParam, mirroring dispatch_fire's \
              own argument-count carve-out, and bundling them would only hide the real reads"
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
    wields: WieldsQuery,
    weapons: Query<(&Magazine, &FireMode, &Handedness)>,
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
                    walking,
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
            AiTarget::new(
                row.entity,
                row_cell(&row.position),
                row_level(&row.position),
            )
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
        if !enemy.life.is_active() || enemy.walking {
            continue;
        }

        let enemy_cell = row_cell(&enemy.position);
        let enemy_level = row_level(&enemy.position);
        let enemy_cell_level = row_cell_level(&enemy.position);

        // (1) ENGAGE — resolve the enemy's weapon (Magazine + single-shot FireModeSpec +
        //     Handedness off the related weapon entity, exactly as dispatch_fire reads it),
        //     then engage iff some opposing ganger passes the SHARED gate: can_see ∧ can_fire
        //     (incl. the GTW-443 hand-count clause folded from the enemy's injury ledger) ∧
        //     ¬Reject arc.
        let weapon_data = wields
            .get(enemy.entity)
            .ok()
            .and_then(Wields::weapon)
            .and_then(|weapon| weapons.get(weapon).ok());
        if let Some((magazine, fire_mode, handedness)) = weapon_data {
            let mode = fire_mode.single();
            let magazine: Magazine = *magazine;
            let handedness: Handedness = *handedness;
            let fire_cost = mode_tu_cost(&mode, &enemy.tu_max, &enemy.aiming, &tuning);
            let observer = Observer {
                position:         &enemy.position,
                stance:           &enemy.stance,
                facing:           &enemy.facing,
                stair_eye_offset: occupancy.stair_eye_offset_at(&enemy_cell_level),
                peek_offset:      PeekOffset::default(),
            };
            let mut engageable: Vec<AiTarget> = Vec::new();
            for target_row in &targets {
                let target_cell = row_cell(&target_row.position);
                let target_level = row_level(&target_row.position);
                let target = Target {
                    position: &target_row.position,
                    stance:   &target_row.stance,
                };
                // can_see (the ONE LOS truth) — conscious observer, in range, clear LOS.
                if !*can_see(
                    &observer,
                    &target,
                    enemy.life,
                    tuning.view_range,
                    &occupancy,
                    &surface,
                    &cover,
                    &tuning,
                    is_dead,
                ) {
                    continue;
                }
                // can_fire (the shared fire guard) — alive, affordable, loaded, in-bounds.
                let actor = FireActor {
                    life: &enemy.life,
                    tu: &enemy.tu,
                    tu_max: &enemy.tu_max,
                    aiming: &enemy.aiming,
                    magazine: &magazine,
                    handedness,
                    hands_available: enemy.hands,
                };
                if !can_fire(&actor, &mode, target_cell, target_level, &tuning) {
                    continue;
                }
                // can_engage (the SHARED ¬Reject arc verdict) — load-bearing for termination:
                // it guarantees the dispatcher will spend TU rather than silently reject.
                if !can_engage(
                    *enemy.facing,
                    enemy_cell,
                    target_cell,
                    enemy.tu,
                    fire_cost,
                    &tuning,
                ) {
                    continue;
                }
                engageable.push(AiTarget::new(target_row.entity, target_cell, target_level));
            }
            if let Some(target) = pick_nearest(enemy_cell, enemy_level, &engageable) {
                fire_writer.write(FireRequested::new(
                    enemy.entity,
                    mode,
                    target.cell,
                    target.level,
                ));
                acted = true;
                continue; // ONE act per ganger per frame.
            }
        }

        // (2) ADVANCE — no engageable target: step toward the nearest opposing ganger's
        //     actual cell over the OmniscientFog move fog (the AI knows where to walk, but
        //     still cannot SHOOT until can_see passes — §D.1/§D.2). For any non-player mover
        //     move_fog provably selects the OmniscientFog, so the brain reads the IDENTICAL
        //     resource dispatch_move's move_fog selects → planner and executor share ONE fog
        //     (the reposition emit-⇒-accept guarantee), without the brain needing the player
        //     fog it would never select. Absent the fog (no live battle) the brain can't
        //     plan and HOLDs.
        if let (Some(goal), Some(omniscient)) = (
            pick_nearest(enemy_cell, enemy_level, &all_targets),
            omniscient.as_deref(),
        ) {
            let relation_of = |occupant: Entity| {
                rows.iter().find(|row| row.entity == occupant).map_or(
                    FactionRelation::Other,
                    |row| {
                        if row.faction == enemy.faction {
                            FactionRelation::OwnSquad
                        } else {
                            FactionRelation::Other
                        }
                    },
                )
            };
            let planning = PlanningView::new(omniscient, relation_of);
            let reachable = reachable_within(
                enemy_cell_level,
                enemy.tu,
                &occupancy,
                &links,
                &tuning,
                &floor_costs,
                enemy.factor,
                &planning,
            );
            if let Some(dest) = plan_advance(enemy_cell_level, goal.cell, &reachable) {
                move_writer.write(MoveRequested::new(enemy.entity, dest));
                acted = true;
            }
        }
        // (3) HOLD — emit nothing.
    }

    // Turn-end, decoupled from emission (§D.3): end the turn ONLY when no enemy acted this
    // frame AND none is mid-walk (a WalkInProgress attached on an earlier frame). `acted`
    // is set only behind a dispatcher-guaranteed-accept pre-check, so an emitted act always
    // spends TU (fire) or attaches a walk (move) — the brain then waits rather than
    // re-emitting, and EndTurnRequested fires only when the enemy genuinely has nothing left.
    let any_walking = enemies.iter().any(|enemy| enemy.walking);
    if !acted && !any_walking {
        end_turn_writer.write(EndTurnRequested);
    }
}
