//! The live reaction-fire trigger system [`reaction_trigger`] and the turn-boundary
//! counter reset [`reset_reactions_used`](super::reset::reset_reactions_used) (GTW-468 — `docs/combat/resolution.md` §8).
//!
//! [`reaction_trigger`] observes the act-in-LOS surface (a completed movement STEP via
//! `Changed<Position>` OR a completed FIRE act via a buffered
//! [`FireDeclaration`](crate::acts::FireDeclaration)), finds every eligible OPPOSING
//! reactor with a clear shot at the acting ganger, runs the GTW-467 opposed check, and on
//! success emits a REAL [`FireRequested`](crate::acts::FireRequested) (the interrupt shot)
//! plus a [`ReactionShotFired`](crate::acts::movement::ReactionShotFired) (halting a walking
//! actor), then increments the reactor's
//! [`ReactionsUsed`] per-turn counter. It reimplements no combat — it WRAPS the landed
//! verbs (`bevy-traps.md` #7 — param-only, no `&mut World`).

use bevy::{
    platform::collections::HashSet,
    prelude::{Changed, Entity, MessageReader, Query, Res, ResMut, With},
};

use super::{
    declared::InterruptSignals,
    interrupt::try_reaction,
    ledger::PendingSpendLedger,
    snapshot::{ReactionGangers, ReactionRow, cell_order},
};
use crate::{
    acts::{FireDeclaration, WeaponProbes},
    cover::CoverLedger,
    fire::WieldsQuery,
    ganger::{LifeState, Position, Suppressed},
    magazine::Magazine,
    occupancy::OccupancyGrid,
    rng::ReactionRng,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ReactionsUsed},
    weapon::{FireMode, Handedness, Silenced, shooter_weapon_silenced},
};

/// The **live reaction-fire trigger** — when a ganger ACTS in an opposing reactor's LOS,
/// run the §8 opposed check and, on success, fire an interrupt + halt a walking actor +
/// consume the reactor's per-turn cap (GTW-468, `docs/combat/resolution.md` §8).
///
/// ## C1 — observe the act-in-LOS surface (faction-agnostic)
///
/// An "act" is detected two ways, and the acting ganger may be player- OR enemy-controlled:
///
/// 1. **a completed movement STEP** — `Changed<Position>` on a ganger. The committed walk
///    ([`advance_walk`](crate::acts::movement::advance_walk)) writes `Position` directly, so a
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
/// ## C2-C4 — see `try_reaction`
///
/// ## C5 — ordering (the critical wiring, `bevy-traps.md` #3)
///
/// This system emits a [`FireRequested`](crate::acts::FireRequested) that
/// [`dispatch_fire`](crate::acts::dispatch_fire)
/// must consume AND a [`ReactionShotFired`](crate::acts::movement::ReactionShotFired) that
/// [`advance_walk`](crate::acts::movement::advance_walk) must consume — both the SAME tick.
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
#[expect(
    clippy::too_many_arguments,
    reason = "the trigger reads the two act-in-LOS surfaces (Changed<Position> movers + the \
              FireDeclaration buffer), the full ganger snapshot, the wielded-weapon + \
              weapon-entity queries (to resolve the reactor's single-shot spec exactly as \
              dispatch_fire does), the shared weapon-marker probe bundle (melee + mounted, \
              GTW-660) + the GTW-526 suppressed-marker probe, the mutable ReactionsUsed \
              counter, the four read grids + tuning the can_see/can_fire/can_engage gates \
              need, the seeded ReactionRng, the GTW-542 silenced-weapon probe, and the two \
              act MessageWriters; each is a distinct Bevy SystemParam, mirroring \
              dispatch_fire's own argument-count carve-out — bundling would only hide the \
              reads"
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
    // GTW-505 C5 / GTW-543 / GTW-660: the SHARED weapon-marker probe bundle (melee +
    // mounted) backing `Wields::firing_weapon` — `reactor_weapon` resolves the SAME
    // weapon `dispatch_fire` will fire (mounted-first, melee-excluded), so a reactor
    // manning an emplacement is gated on the MOUNTED gun's mode / TU / magazine and an
    // interrupt never fires the melee weapon. The same `WeaponProbes` bundle
    // `dispatch_fire` takes (two disjoint unit-item marker probes).
    probes: WeaponProbes,
    // GTW-526 C3: the SUPPRESSED-reactor probe — a read-only marker query so the
    // eligibility gate can skip a suppressed reactor BEFORE the interrupt roll, consuming
    // ZERO ReactionRng draws (determinism-critical: a suppressed unit must not perturb the
    // RNG stream). A `Query<(), With<Suppressed>>`, disjoint from every other param.
    suppressed: Query<(), With<Suppressed>>,
    // GTW-542 / GTW-674: the SILENCED-weapon probe — a read-only marker query so an ACT by a
    // shooter wielding a silenced weapon does NOT trip a reaction (a suppressor removes the
    // shot's reveal). Gated on the ACTOR (the declaration's shooter), resolved via the shared
    // firing-weapon rule (`shooter → Wields → the FIRING weapon`, mounted-first via
    // `probes.mounted`, melee-excluded), so a MOUNTED shooter's reveal tracks the mount it
    // fires and a silenced INTERRUPT shot also stays silent. A `Query<(), With<Silenced>>`,
    // disjoint from every other param.
    silenced: Query<(), With<Silenced>>,
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
    // C4 + GTW-727 C5: the interrupt shot, the walking-actor halt, and the interrupt
    // exposure signal — bundled into ONE SystemParam so this system stays under Bevy's
    // 16-param arity (the `FireSignals` bundling precedent).
    mut signals: InterruptSignals,
) {
    // bevy-traps.md #1: without the seeded reaction stream no opposed check can roll, so no
    // interrupt can fire — fail closed (no panic) rather than reading an absent resource. In
    // the real app the stream is always present (sim-set on the setup Ok path).
    let Some(mut rng) = rng else {
        return;
    };

    // Snapshot every ganger as a Copy row so the closures below borrow the snapshot (not the
    // live query) and every ordering is a pure function of game state (bevy-traps.md #3).
    // GTW-646: reactor GATE reads overlay this settled snapshot with the pass ledger below
    // (the actor-side reads stay settled — the act happened at its settled state).
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
        // GTW-542: a SILENCED shot does not reveal — a FireDeclaration whose shooter wields a
        // silenced weapon is NOT an act-in-LOS event, so it never enters the actor set (no
        // reactor rolls against it, no ReactionRng draws). A moving shooter still reveals via
        // the Changed<Position> surface above; only the shot's noise is removed. A silenced
        // INTERRUPT shot's own FireDeclaration is gated the same way (it stays silent too).
        if *shooter_weapon_silenced(
            declaration.shooter,
            &wields,
            &probes.mounted,
            &probes.melee,
            &silenced,
        ) {
            continue;
        }
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

    // GTW-646: the pass's pending-spend ledger. Each emitted interrupt's predicted
    // post-dispatch TU / facing / magazine is folded in, so a reactor evaluated AGAIN
    // this pass (a second actor in the same tick) is gated on the state `dispatch_fire`
    // will actually see — the cap spend stays 1:1 with actually-dispatched shots.
    let mut ledger = PendingSpendLedger::default();

    for actor in &acting_rows {
        // A dead/downed actor is not a live act-in-LOS event — fail closed (only an Alive
        // ganger moving/firing crosses a sightline as an act to react to).
        if !*actor.life.is_active() {
            continue;
        }
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
            if let Some(commit) = try_reaction(
                actor,
                reactor,
                &ledger,
                &wields,
                &weapons,
                &probes,
                &suppressed,
                &mut used,
                &tuning,
                &occupancy,
                &surface,
                &cover,
                &mut rng,
                is_dead,
                &mut signals,
            ) {
                ledger.commit(commit);
            }
        }
    }
}
