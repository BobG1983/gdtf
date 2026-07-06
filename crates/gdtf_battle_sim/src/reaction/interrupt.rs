//! The per-(actor, reactor) interrupt evaluation — the C2 eligibility gates, the
//! C3 opposed check, and the C4 fire + halt + count emission (GTW-468).

use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::snapshot::{ReactionRow, row_cell_level};
use crate::{
    acts::{FireRequested, can_engage, movement::ReactionShotFired},
    cover::CoverLedger,
    fire::{MeleeQuery, WieldsQuery},
    ganger::Suppressed,
    injuries::HandsAvailable,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire, mode_tu_cost},
    occupancy::OccupancyGrid,
    rng::ReactionRng,
    surface::SurfaceGrid,
    tuning::{
        CombatTuning, ReactionsUsed, interrupt_probability, may_interrupt, reaction_score,
        rolls_interrupt,
    },
    weapon::{FireMode, Handedness},
};

/// Evaluate ONE `(actor, reactor)` pair — gate the reactor's eligibility, run the
/// §8 opposed check, and on success fire the interrupt, halt the walking actor, and
/// consume the reactor's per-turn cap (extracted from [`reaction_trigger`](super::trigger::reaction_trigger)
/// so that system stays under the line cap; every gate that failed `continue`s here
/// as an early `return` — one evaluation per call, identical behavior).
///
/// ## C2 — eligible reactors, gated
///
/// For each acting ganger, a candidate REACTOR is a ganger that is: ALIVE/conscious
/// ([`LifeState::is_active`](crate::ganger::LifeState::is_active)); of the OPPOSING faction
/// to the actor; with unspent TU (`Tu > 0`); NOT
/// [`Suppressed`](crate::ganger::Suppressed) (GTW-526 C3 — a pinned unit keeps its head
/// down, skipped BEFORE the interrupt roll so it consumes zero
/// [`ReactionRng`](crate::rng::ReactionRng) draws); with cap room
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
/// [`single`]: crate::weapon::FireMode::single
#[expect(
    clippy::too_many_arguments,
    reason = "the per-pair evaluation borrows the trigger system's own params (the \
              wielded-weapon + weapon-entity queries, the suppressed probe, the mutable \
              ReactionsUsed counter, the four read grids + tuning, the seeded ReactionRng, \
              the is_dead corpse predicate, and the two act MessageWriters); each is a \
              distinct borrow mirroring reaction_trigger's own argument-count carve-out — \
              bundling would only hide the reads"
)]
pub(super) fn try_reaction(
    actor: &ReactionRow,
    reactor: &ReactionRow,
    wields: &WieldsQuery,
    weapons: &Query<(&Magazine, &FireMode, &Handedness)>,
    melee: &MeleeQuery,
    suppressed: &Query<(), With<Suppressed>>,
    used: &mut Query<&mut ReactionsUsed>,
    tuning: &CombatTuning,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    rng: &mut ReactionRng,
    is_dead: &impl Fn(Entity) -> bool,
    fire_writer: &mut MessageWriter<FireRequested>,
    halt_writer: &mut MessageWriter<ReactionShotFired>,
) {
    // The canonical CellLevel accessors through Position's deref (GTW-565).
    let actor_cell = actor.position.cell();
    let actor_level = actor.position.level();

    // C2: eligibility gate — alive + unspent TU + cap room. The cap read is the
    // reactor's LIVE ReactionsUsed (mutated by an earlier successful interrupt this
    // same tick), so a reactor that already hit its cap this pass is refused.
    if !reactor.life.is_active() || *reactor.tu == 0 {
        return;
    }
    // GTW-526 C3: a SUPPRESSED reactor cannot interrupt — a pinned unit is a worse
    // reactor (it keeps its head down). This skip happens in the ELIGIBILITY gate,
    // BEFORE the opposed check's `rolls_interrupt` draw (below), so a suppressed
    // reactor consumes ZERO `ReactionRng` draws — the RNG stream is IDENTICAL to a
    // run where the reactor is simply absent. Determinism-critical: we never
    // draw-then-discard (which would perturb every later reactor's roll); the
    // suppression check gates purely on the marker, no RNG touched.
    if suppressed.get(reactor.entity).is_ok() {
        return;
    }
    let used_now = used
        .get(reactor.entity)
        .copied()
        .unwrap_or_else(|_| ReactionsUsed::new(0));
    if !may_interrupt(used_now, reactor.reactions, &tuning.reaction) {
        return;
    }

    // Resolve the reactor's weapon EXACTLY as dispatch_fire / enemy_ai_turn do
    // (`ganger → Wields → the weapon entity`) — the single-shot spec + Magazine +
    // Handedness the can_fire gate + the FireRequested need. A reactor wielding no
    // weapon (or whose weapon entity is missing) cannot react — fail closed.
    let Some((mode, magazine, handedness)) = reactor_weapon(reactor.entity, wields, weapons, melee)
    else {
        return;
    };
    let fire_cost = mode_tu_cost(&mode, &reactor.tu_max, &reactor.aiming, tuning);

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
        occupancy,
        surface,
        cover,
        tuning,
        is_dead,
    ) {
        return;
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
    if !can_fire(&fire_actor, &mode, actor_cell, actor_level, tuning) {
        return;
    }
    // can_engage (the SHARED ¬Reject arc verdict) — the reactor turns-to-fire if it
    // can afford turn + shot, else the interrupt would be rejected (so don't emit it).
    if !can_engage(
        *reactor.facing,
        reactor.position.cell(),
        actor_cell,
        reactor.tu,
        fire_cost,
        tuning,
    ) {
        return;
    }

    // C3: the opposed check — score both sides, derive the clamped probability, roll
    // ONE seeded draw. The watcher is the reactor, the mover is the actor.
    let watcher_score = reaction_score(reactor.reactions, reactor.tu, reactor.tu_max);
    let mover_score = reaction_score(actor.reactions, actor.tu, actor.tu_max);
    let probability = interrupt_probability(watcher_score, mover_score, &tuning.reaction);
    if !rolls_interrupt(probability, rng) {
        return;
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

/// Resolve a reactor's single-shot fire spec + magazine + handedness through
/// `ganger → Wields → the weapon entity` — the EXACT traversal
/// [`dispatch_fire`](crate::acts::dispatch_fire) / `enemy_ai_turn` use, so the interrupt
/// shot fires the same weapon the dispatcher would (GTW-468 C9 — reuse, don't reimplement).
///
/// `None` when the reactor wields no weapon or its weapon entity is missing from the weapon
/// query — the reactor then cannot react (fail closed).
pub(super) fn reactor_weapon(
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
