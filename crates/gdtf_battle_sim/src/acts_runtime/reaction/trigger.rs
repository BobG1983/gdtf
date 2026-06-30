//! The live reaction-fire trigger system [`reaction_trigger`] and the turn-boundary
//! counter reset [`reset_reactions_used`] (GTW-468 — `docs/combat/resolution.md` §8).
//!
//! [`reaction_trigger`] observes the act-in-LOS surface (a completed movement STEP via
//! `Changed<Position>` OR a completed FIRE act via a buffered
//! [`FireDeclaration`](crate::acts::FireDeclaration)), finds every eligible OPPOSING
//! reactor with a clear shot at the acting ganger, runs the GTW-467 opposed check, and on
//! success emits a REAL [`FireRequested`](crate::acts::FireRequested) (the interrupt shot)
//! plus a [`ReactionShotFired`] (halting a walking actor), then increments the reactor's
//! [`ReactionsUsed`] per-turn counter. It reimplements no combat — it WRAPS the landed
//! verbs (`bevy-traps.md` #7 — param-only, no `&mut World`).

use bevy::{
    platform::collections::HashSet,
    prelude::{Changed, Entity, MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::{FireDeclaration, FireRequested, can_engage},
    cover::CoverLedger,
    fire::{MeleeQuery, WieldsQuery},
    ganger::{Aiming, Facing, Faction, LifeState, Position, Reactions, Stance, Tu, TuMax},
    injuries::HandsAvailable,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire, mode_tu_cost},
    metric::{Cell, CellLevel, Level},
    move_acts::ReactionShotFired,
    occupancy::OccupancyGrid,
    rng::ReactionRng,
    surface::SurfaceGrid,
    tuning::{
        CombatTuning, ReactionsUsed, interrupt_probability, may_interrupt, reaction_score,
        rolls_interrupt,
    },
    weapon::{FireMode, Handedness},
};

/// One ganger's reaction-relevant snapshot — the Copy row [`reaction_trigger`] reads out of
/// the live query so the opposed check closes over plain data in a deterministic order,
/// never raw `Query` iteration order (`bevy-traps.md` #3, the `enemy_ai_turn` `GangerRow`
/// precedent).
///
/// Holds BOTH a reactor's gate inputs (its position / stance / facing / aim / life / TU /
/// `Reactions`) AND an actor's geometry (position / stance), since every ganger may be both
/// (faction-agnostic — the actor may be player- or enemy-controlled, C1).
#[derive(Clone, Copy)]
struct ReactionRow {
    /// The ganger entity — the act emission's actor/reactor ref.
    entity:    Entity,
    /// Its `(cell, level)` grid position — the eye / aim / arc datum.
    position:  Position,
    /// Its stance — the eye / silhouette anchor for [`can_see`].
    stance:    Stance,
    /// Its facing — the arc datum for [`can_engage`].
    facing:    Facing,
    /// Its aim mode — the per-shot TU-premium selector for the interrupt shot's cost.
    aiming:    Aiming,
    /// Its life state — only an Alive reactor watches; only an Alive ganger is a valid
    /// reaction target (a corpse/Downed body never reacts).
    life:      LifeState,
    /// Its current TU pool — a reactor needs `Tu > 0` (unspent TU funds the interrupt).
    tu:        Tu,
    /// Its round-start TU ceiling — the denominator of the §8 reaction score.
    tu_max:    TuMax,
    /// Its gang — splits each `(reactor, actor)` pair into OPPOSING factions.
    faction:   Faction,
    /// Its derived `Reactions` stat — the §8 score numerator + the cap input.
    reactions: Reactions,
}

/// The ground cell `(x, y)` of a [`Position`] — the z storey dropped (the `fire.rs`
/// `actor_cell` / `brain.rs` `row_cell` split precedent).
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
                  u8 narrowing cannot truncate or sign-flip (the brain.rs row_level precedent)"
    )]
    let storey = key.z as u8;
    Level::new(storey)
}

/// The `(cell, level)` key of a [`Position`].
fn row_cell_level(position: &Position) -> CellLevel {
    **position
}

/// The `(level, y, x)` sort key of a [`Position`] — the deterministic total order the
/// trigger evaluates reactors in (`bevy-traps.md` #3 / the `brain.rs` `cell_order`
/// precedent), so the [`ReactionRng`] draws consume in a reproducible order.
fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

/// The read-only reaction snapshot query shape — every ganger's gate-relevant components,
/// factored into a `type` so the system signature stays under clippy's type-complexity gate
/// (the `EnemyTurnGangers` precedent).
type ReactionGangers<'world, 'state> = Query<
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
        &'static Reactions,
    ),
>;

/// The **live reaction-fire trigger** — when a ganger ACTS in an opposing reactor's LOS,
/// run the §8 opposed check and, on success, fire an interrupt + halt a walking actor +
/// consume the reactor's per-turn cap (GTW-468, `docs/combat/resolution.md` §8).
///
/// ## C1 — observe the act-in-LOS surface (faction-agnostic)
///
/// An "act" is detected two ways, and the acting ganger may be player- OR enemy-controlled:
///
/// 1. **a completed movement STEP** — `Changed<Position>` on a ganger. The committed walk
///    ([`advance_walk`](crate::move_acts::advance_walk)) writes `Position` directly, so a
///    step trips `Changed<Position>`. Because this system runs `.before(advance_walk)`
///    (C5), the change it reads is the PRIOR tick's settled step (Bevy change-detection
///    persists one cycle); and
/// 2. **a completed FIRE act** — a buffered [`FireDeclaration`] (one per proceeding shot;
///    `dispatch_fire` emits it). The declared shooter is the actor. (Reading the
///    declaration rather than the per-round [`ShotFired`](crate::shot_fired::ShotFired)
///    naturally de-duplicates a burst to one act.)
///
/// Turn / stance / reload acts are **excluded** — they do not cross an LOS the way a step or
/// a shot does (DESIGN FORK a, flagged not silently chosen).
///
/// ## C2 — eligible reactors, gated
///
/// For each acting ganger, a candidate REACTOR is a ganger that is: ALIVE/conscious
/// ([`LifeState::is_active`](crate::ganger::LifeState::is_active)); of the OPPOSING faction
/// to the actor; with unspent TU (`Tu > 0`); with cap room
/// ([`may_interrupt`](crate::tuning::may_interrupt) true). Each `(reactor, actor)` pair is
/// then gated with the EXISTING faction-agnostic [`can_see`] (conscious-observer + range +
/// LOS) and [`can_fire`] + [`can_engage`] (loaded + affordable + the shared arc verdict) so
/// the emitted interrupt is dispatcher-accepted. Reactors are evaluated in the deterministic
/// `(level, y, x)` order; each reactor gets AT MOST ONE roll per actor-act, subject to its
/// own cap (DESIGN FORK b).
///
/// ## C3 — the opposed check
///
/// For an eligible, sighted pair: [`reaction_score`](crate::tuning::reaction_score) of the
/// watcher and the mover, then [`interrupt_probability`](crate::tuning::interrupt_probability),
/// then [`rolls_interrupt`](crate::tuning::rolls_interrupt) — ONE seeded
/// [`ReactionRng`](crate::rng::ReactionRng) draw. Same `BattleSeed` ⇒ identical interrupt
/// sequence (the deterministic-order evaluation above keeps the draw stream replay-stable).
///
/// ## C4 — on success: fire + halt + count
///
/// Emit a REAL [`FireRequested`] from the reactor at the actor's cell/level in [`single`]
/// mode (so `dispatch_fire` → `fire()` resolves a NORMAL shot — full dispersion + damage,
/// **no reaction damage modifier**, AC7 — AND spends the reactor's TU, so this system never
/// double-charges); AND emit [`ReactionShotFired`] `{ mover: actor }` so a walking actor
/// halts at its current cell (closing the GTW-355 orphan); AND
/// [`increment`](crate::tuning::ReactionsUsed::increment) the reactor's [`ReactionsUsed`].
///
/// ## C5 — ordering (the critical wiring, `bevy-traps.md` #3)
///
/// This system emits a [`FireRequested`] that [`dispatch_fire`](crate::acts::dispatch_fire)
/// must consume AND a [`ReactionShotFired`] that
/// [`advance_walk`](crate::move_acts::advance_walk) must consume — both the SAME tick.
/// `dispatch_fire` runs EARLY in the Simulate band; `advance_walk` runs LATE
/// (`.after(dispatch_move)`). A system cannot be `.after(advance_walk)` AND
/// `.before(dispatch_fire)` in one frame — that is a cycle. The resolution: this system
/// detects the COMPLETED act from change-detection / the buffered declaration of the PRIOR
/// settle, and is ordered `.before(dispatch_fire)` AND `.before(advance_walk)` (wired in
/// [`SimActsPlugin`](crate::acts::SimActsPlugin)). So both messages it writes are consumed
/// the SAME tick they are written — a documented, tested one-tick cadence between an act and
/// its interrupt (the `advance_walk` one-step-per-tick / GTW-461 act-cadence model). AC1
/// (the shot fires) + AC2 (the walk halts) are the proof it dispatches coherently.
///
/// Param-only (`Query` / `Res` / `ResMut` / `MessageReader` / `MessageWriter`) — no
/// `&mut World` (`bevy-traps.md` #7). The mutable [`ReactionsUsed`] query is disjoint from
/// the read snapshot (a different component), so no `ParamSet` is needed.
///
/// [`single`]: crate::weapon::FireMode::single
#[expect(
    clippy::too_many_arguments,
    reason = "the trigger reads the two act-in-LOS surfaces (Changed<Position> movers + the \
              FireDeclaration buffer), the full ganger snapshot, the wielded-weapon + \
              weapon-entity queries (to resolve the reactor's single-shot spec exactly as \
              dispatch_fire does), the mutable ReactionsUsed counter, the four read grids + \
              tuning the can_see/can_fire/can_engage gates need, the seeded ReactionRng, and \
              the two act MessageWriters; each is a distinct Bevy SystemParam, mirroring \
              dispatch_fire's own argument-count carve-out — bundling would only hide the \
              reads"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the trigger is ONE cohesive per-tick pass (collect the acting gangers from \
              the two surfaces → for each, evaluate every eligible reactor in deterministic \
              order → fire + halt + count on a successful opposed check); splitting it would \
              thread the grids/tuning/snapshot + the borrowed is_dead closure through helpers \
              and obscure the access set more than the length costs (the enemy_ai_turn \
              too_many_lines precedent)"
)]
pub fn reaction_trigger(
    // C1 surface (a): gangers whose Position CHANGED — a completed movement step. Filtered
    // to the moved entities only (a `Changed<Position>` query), so a still ganger costs
    // nothing. `Without<ReactionsUsed>` is NOT applied — every ganger carries the counter.
    moved: Query<Entity, Changed<Position>>,
    // C1 surface (b): the FIRE-act declarations buffered last tick — the declared shooter is
    // the actor. MessageReader (NOT EventReader — bevy-traps.md #4).
    mut declarations: MessageReader<FireDeclaration>,
    // The full ganger snapshot — the reactor gate inputs AND the actor geometry.
    gangers: ReactionGangers,
    // The reactor's wielded weapon, resolved EXACTLY as dispatch_fire/enemy_ai_turn do
    // (`ganger → Wields → the weapon entity`), to read the single-shot FireModeSpec + the
    // Magazine + Handedness the can_fire gate + the FireRequested need.
    wields: WieldsQuery,
    weapons: Query<(&Magazine, &FireMode, &Handedness)>,
    // GTW-505 C5: the melee-weapon marker probe — `reactor_weapon` resolves the RANGED
    // weapon (excluding the melee weapon the reactor also wields) so an interrupt fires the
    // reactor's GUN, never its melee weapon (a `Query<(), With<MeleeWeapon>>`, disjoint).
    melee: MeleeQuery,
    // C4: the per-turn interrupt counter, mutated through its own `increment` (a different
    // component than the read snapshot, so this &mut query is disjoint — no ParamSet).
    mut used: Query<&mut ReactionsUsed>,
    // The gate inputs the can_see / can_fire / can_engage checks need.
    tuning: Res<CombatTuning>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    // C3: the seeded reaction stream — ONE draw per opposed check (ResMut, never Res —
    // drawing advances the cursor; rng::streams binding constraint). `ReactionRng` IS sim-set
    // (inserted by `setup_battle_on_request` alongside the other four streams, sharing the
    // BattleInProgress lifetime), so in the real app it is always present when this gated
    // band runs. Taken `Option<ResMut<…>>` so a sim-only harness that opens BattleInProgress
    // WITHOUT the full setup flow (the bleed/turn/input runtime tests do) does not panic this
    // runtime system on the stream's absence (bevy-traps.md #1, the `dispatch_fire`
    // `Option<Res<InjuryTables>>` precedent). With it ABSENT no opposed check can roll, so no
    // interrupt fires — a safe, defined fallback, never a panic.
    rng: Option<ResMut<ReactionRng>>,
    // C4: the interrupt shot + the walking-actor halt.
    mut fire_writer: MessageWriter<FireRequested>,
    mut halt_writer: MessageWriter<ReactionShotFired>,
) {
    // bevy-traps.md #1: without the seeded reaction stream no opposed check can roll, so no
    // interrupt can fire — fail closed (no panic) rather than reading an absent resource. In
    // the real app the stream is always present (sim-set on the setup Ok path).
    let Some(mut rng) = rng else {
        return;
    };

    // Snapshot every ganger as a Copy row so the closures below borrow the snapshot (not the
    // live query) and every ordering is a pure function of game state (bevy-traps.md #3).
    let rows: Vec<ReactionRow> = gangers
        .iter()
        .map(
            |(entity, position, stance, facing, aiming, life, tu, tu_max, faction, reactions)| {
                ReactionRow {
                    entity,
                    position: *position,
                    stance: *stance,
                    facing: *facing,
                    aiming: *aiming,
                    life: *life,
                    tu: *tu,
                    tu_max: *tu_max,
                    faction: *faction,
                    reactions: *reactions,
                }
            },
        )
        .collect();

    // C1: the set of acting ganger entities — the union of the two surfaces:
    //  (a) every ganger whose Position changed (a completed step), and
    //  (b) every shooter named in a FireDeclaration this tick (a completed fire act).
    // A HashSet de-duplicates an actor that both moved and fired in the same window, so each
    // actor-act is evaluated once. (The set is iterated below in the snapshot's deterministic
    // order, NOT HashSet order, so determinism holds.)
    let mut actors: HashSet<Entity> = moved.iter().collect();
    for declaration in declarations.read() {
        actors.insert(declaration.shooter);
    }
    if actors.is_empty() {
        return;
    }

    // A Dead ganger is a corpse the LOS march flies THROUGH (GTW-317) — the same `is_dead`
    // pass-through can_see / has_los take. An entity absent from the snapshot is no corpse.
    let is_dead_fn = |entity: Entity| {
        rows.iter()
            .find(|row| row.entity == entity)
            .is_some_and(|row| row.life == LifeState::Dead)
    };
    let is_dead = &is_dead_fn;

    // Evaluate each acting ganger in the snapshot's deterministic (level, y, x) order, so the
    // ReactionRng draw stream is replay-stable regardless of HashSet iteration order (C3).
    let mut acting_rows: Vec<ReactionRow> = rows
        .iter()
        .copied()
        .filter(|row| actors.contains(&row.entity))
        .collect();
    acting_rows.sort_by_key(|row| cell_order(&row.position));

    for actor in &acting_rows {
        // A dead/downed actor is not a live act-in-LOS event — fail closed (only an Alive
        // ganger moving/firing crosses a sightline as an act to react to).
        if !actor.life.is_active() {
            continue;
        }
        let actor_cell = row_cell(&actor.position);
        let actor_level = row_level(&actor.position);

        // The candidate REACTORS for this actor, in deterministic (level, y, x) order.
        let mut reactors: Vec<ReactionRow> = rows
            .iter()
            .copied()
            // OPPOSING faction (C2) — a same-gang ganger never reacts to its ally; and never
            // the actor itself.
            .filter(|row| row.faction != actor.faction && row.entity != actor.entity)
            .collect();
        reactors.sort_by_key(|row| cell_order(&row.position));

        for reactor in &reactors {
            // C2: eligibility gate — alive + unspent TU + cap room. The cap read is the
            // reactor's LIVE ReactionsUsed (mutated by an earlier successful interrupt this
            // same tick), so a reactor that already hit its cap this pass is refused.
            if !reactor.life.is_active() || *reactor.tu == 0 {
                continue;
            }
            let used_now = used
                .get(reactor.entity)
                .copied()
                .unwrap_or_else(|_| ReactionsUsed::new(0));
            if !may_interrupt(used_now, reactor.reactions, &tuning.reaction) {
                continue;
            }

            // Resolve the reactor's weapon EXACTLY as dispatch_fire / enemy_ai_turn do
            // (`ganger → Wields → the weapon entity`) — the single-shot spec + Magazine +
            // Handedness the can_fire gate + the FireRequested need. A reactor wielding no
            // weapon (or whose weapon entity is missing) cannot react — fail closed.
            let Some((mode, magazine, handedness)) =
                reactor_weapon(reactor.entity, &wields, &weapons, &melee)
            else {
                continue;
            };
            let fire_cost = mode_tu_cost(&mode, &reactor.tu_max, &reactor.aiming, &tuning);

            // C2: the per-pair LOS + arc + fire gates — REUSED verbatim (faction-agnostic),
            // so a fired interrupt is dispatcher-accepted.
            let observer = Observer {
                position:         &reactor.position,
                stance:           &reactor.stance,
                facing:           &reactor.facing,
                stair_eye_offset: occupancy.stair_eye_offset_at(&row_cell_level(&reactor.position)),
                peek_offset:      PeekOffset::default(),
            };
            let target = Target {
                position: &actor.position,
                stance:   &actor.stance,
            };
            // can_see (the ONE LOS truth) — conscious observer, in range, clear LOS.
            if !*can_see(
                &observer,
                &target,
                reactor.life,
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
            let fire_actor = FireActor {
                life: &reactor.life,
                tu: &reactor.tu,
                tu_max: &reactor.tu_max,
                aiming: &reactor.aiming,
                magazine: &magazine,
                handedness,
                hands_available: HandsAvailable::default(),
            };
            if !can_fire(&fire_actor, &mode, actor_cell, actor_level, &tuning) {
                continue;
            }
            // can_engage (the SHARED ¬Reject arc verdict) — the reactor turns-to-fire if it
            // can afford turn + shot, else the interrupt would be rejected (so don't emit it).
            if !can_engage(
                *reactor.facing,
                row_cell(&reactor.position),
                actor_cell,
                reactor.tu,
                fire_cost,
                &tuning,
            ) {
                continue;
            }

            // C3: the opposed check — score both sides, derive the clamped probability, roll
            // ONE seeded draw. The watcher is the reactor, the mover is the actor.
            let watcher_score = reaction_score(reactor.reactions, reactor.tu, reactor.tu_max);
            let mover_score = reaction_score(actor.reactions, actor.tu, actor.tu_max);
            let probability = interrupt_probability(watcher_score, mover_score, &tuning.reaction);
            if !rolls_interrupt(probability, &mut rng) {
                continue;
            }

            // C4: SUCCESS — fire the interrupt, halt the walking actor, count the interrupt.
            //  (i) the REAL interrupt shot (single mode) — dispatch_fire → fire() resolves a
            //      NORMAL shot (full dispersion + damage, no reaction modifier — AC7) AND
            //      spends the reactor's TU (so we never double-charge here).
            fire_writer.write(FireRequested::new(
                reactor.entity,
                mode,
                actor_cell,
                actor_level,
            ));
            //  (ii) halt a walking actor at its current cell (the GTW-355 orphan's producer).
            //       A non-walking actor's advance_walk read is a harmless no-op.
            halt_writer.write(ReactionShotFired::new(actor.entity));
            //  (iii) consume the reactor's per-turn cap so the next may_interrupt sees it —
            //       both for a later actor this tick AND the LIVE read above for this same pass.
            if let Ok(mut counter) = used.get_mut(reactor.entity) {
                counter.increment();
            }
        }
    }
}

/// Resolve a reactor's single-shot fire spec + magazine + handedness through
/// `ganger → Wields → the weapon entity` — the EXACT traversal
/// [`dispatch_fire`](crate::acts::dispatch_fire) / `enemy_ai_turn` use, so the interrupt
/// shot fires the same weapon the dispatcher would (GTW-468 C9 — reuse, don't reimplement).
///
/// `None` when the reactor wields no weapon or its weapon entity is missing from the weapon
/// query — the reactor then cannot react (fail closed).
fn reactor_weapon(
    reactor: Entity,
    wields: &WieldsQuery,
    weapons: &Query<(&Magazine, &FireMode, &Handedness)>,
    melee: &MeleeQuery,
) -> Option<(crate::weapon::FireModeSpec, Magazine, Handedness)> {
    // GTW-505 C5: resolve the RANGED weapon (excluding the melee weapon the reactor also
    // wields) so the interrupt fires the gun, never the melee weapon — the same
    // ranged-filtered resolution `dispatch_fire` / `fire()` use.
    let weapon_entity = wields
        .get(reactor)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))?;
    let (magazine, fire_mode, handedness) = weapons.get(weapon_entity).ok()?;
    Some((fire_mode.single(), *magazine, *handedness))
}

/// **Reset** every watcher's per-turn interrupt counter at a turn boundary — the §8 cap
/// "max interrupts this enemy turn" applies AFRESH each turn (GTW-468 C6).
///
/// Drains [`MessageReader<TurnStarted>`] (the turn-cycle boundary
/// [`dispatch_end_turn`](crate::turn::dispatch_end_turn) emits per advance) and, when ANY
/// turn started this tick, zeroes every ganger's [`ReactionsUsed`]
/// ([`reset`](crate::tuning::ReactionsUsed::reset)).
///
/// **Why reset every boundary (not only the enemy's).** §8 frames the cap as "this enemy
/// turn", but reaction fire is faction-symmetric (AC5 — a player watcher reacts on the enemy
/// turn AND an enemy watcher reacts on the player turn). The cap is per-WATCHER and a
/// watcher only reacts during its OPPONENT's turn, so resetting EVERY watcher at EVERY turn
/// boundary generalizes "this enemy turn" to "this turn relative to the watcher" correctly:
/// a watcher's counter is zeroed before each turn in which it could react, and never
/// double-counts across two of its own opponent's turns. This is the defensible default
/// (DESIGN FORK, flagged); a per-faction reset would be a no-op refinement.
///
/// Ordered `.after(`[`dispatch_end_turn`](crate::turn::dispatch_end_turn)`)` (so the
/// boundary's [`TurnStarted`](crate::turn::TurnStarted) is buffered) in
/// [`SimActsPlugin`](crate::acts::SimActsPlugin). Its own independent reader, so it never
/// steals the boundary from the combat-log / bleed readers (the `tick_bleed`
/// `enemy_phase_started` precedent). Param-only — `Query` / `MessageReader`, no `&mut World`
/// (`bevy-traps.md` #7).
pub fn reset_reactions_used(
    mut turns: MessageReader<crate::turn::TurnStarted>,
    mut used: Query<&mut ReactionsUsed>,
) {
    // A turn boundary crossed this tick iff any TurnStarted was emitted. The reset is
    // idempotent (zeroing is the same for N boundaries as for one), so drain-then-act-once.
    let mut crossed = false;
    for _turn in turns.read() {
        crossed = true;
    }
    if !crossed {
        return;
    }
    for mut counter in &mut used {
        counter.reset();
    }
}
