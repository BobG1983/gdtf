//! The per-(actor, reactor) interrupt evaluation — the C2 eligibility gates, the
//! C3 opposed check, and the C4 fire + halt + count emission (GTW-468).

use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::{
    ledger::{InterruptCommit, PendingSpendLedger},
    snapshot::{ReactionRow, row_cell_level},
};
use crate::{
    acts::{
        FireArcDecision, FireRequested, WeaponProbes, decide_fire_arc, movement::ReactionShotFired,
    },
    cover::CoverLedger,
    fire::WieldsQuery,
    ganger::{Facing, Suppressed, Tu},
    injuries::HandsAvailable,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, Magazine, can_fire, clamp_burst, mode_tu_cost},
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
/// LOS) and [`can_fire`] + [`decide_fire_arc`] (loaded + affordable + the shared arc
/// verdict `dispatch_fire` itself matches) so the emitted interrupt is
/// dispatcher-accepted. Reactors are evaluated in the deterministic `(level, y, x)` order;
/// each reactor gets AT MOST ONE roll per actor-act, subject to its own cap (DESIGN FORK
/// b).
///
/// GTW-646 (spend integrity): the TU / facing / magazine these gates read come through
/// the trigger pass's [`PendingSpendLedger`] — the settled snapshot OVERLAID with every
/// interrupt already emitted this pass — so a SECOND same-pass interrupt is gated on the
/// state the dispatcher will actually see, never a stale snapshot. An offer the
/// dispatcher could not accept is skipped HERE, before the opposed-check roll (zero
/// [`ReactionRng`](crate::rng::ReactionRng) draws, the GTW-526 suppression-skip shape),
/// keeping the C4 cap spend 1:1 with actually-dispatched shots.
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
/// The returned [`InterruptCommit`] carries the predicted post-dispatch TU / facing /
/// magazine (computed from the SAME shared sources the dispatcher runs —
/// [`decide_fire_arc`] / [`mode_tu_cost`] / the [`Magazine`] round spend) for the trigger
/// to fold into the pass ledger; `None` means no interrupt was emitted (a failed gate or
/// a failed roll).
///
/// [`single`]: crate::weapon::FireMode::single
#[expect(
    clippy::too_many_arguments,
    reason = "the per-pair evaluation borrows the trigger system's own params (the \
              wielded-weapon + weapon-entity queries, the weapon-marker probe bundle, the \
              suppressed probe, the pass's pending-spend ledger, the mutable ReactionsUsed \
              counter, the four read grids + tuning, the seeded ReactionRng, the is_dead \
              corpse predicate, and the two act MessageWriters); each is a distinct borrow \
              mirroring reaction_trigger's own argument-count carve-out — bundling would \
              only hide the reads"
)]
pub(super) fn try_reaction(
    actor: &ReactionRow,
    reactor: &ReactionRow,
    ledger: &PendingSpendLedger,
    wields: &WieldsQuery,
    weapons: &Query<(&Magazine, &FireMode, &Handedness)>,
    probes: &WeaponProbes,
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
) -> Option<InterruptCommit> {
    // The canonical CellLevel accessors through Position's deref (GTW-565).
    let actor_cell = actor.position.cell();
    let actor_level = actor.position.level();

    // GTW-646: the reactor's WORKING TU / facing — the settled snapshot overlaid with
    // every interrupt already committed this pass, so a second same-pass evaluation
    // gates on what the dispatcher will actually see (never a stale snapshot).
    let tu_now = ledger.tu_of(reactor.entity, reactor.tu);
    let facing_now = ledger.facing_of(reactor.entity, reactor.facing);

    // C2: eligibility gate — alive + unspent TU + cap room. The cap read is the
    // reactor's LIVE ReactionsUsed (mutated by an earlier successful interrupt this
    // same tick), so a reactor that already hit its cap this pass is refused.
    if !*reactor.life.is_active() || *tu_now == 0 {
        return None;
    }
    // GTW-526 C3: a SUPPRESSED reactor cannot interrupt — a pinned unit is a worse
    // reactor (it keeps its head down). This skip happens in the ELIGIBILITY gate,
    // BEFORE the opposed check's `rolls_interrupt` draw (below), so a suppressed
    // reactor consumes ZERO `ReactionRng` draws — the RNG stream is IDENTICAL to a
    // run where the reactor is simply absent. Determinism-critical: we never
    // draw-then-discard (which would perturb every later reactor's roll); the
    // suppression check gates purely on the marker, no RNG touched.
    if suppressed.get(reactor.entity).is_ok() {
        return None;
    }
    let used_now = used
        .get(reactor.entity)
        .copied()
        .unwrap_or_else(|_| ReactionsUsed::new(0));
    if !*may_interrupt(used_now, reactor.reactions, &tuning.reaction) {
        return None;
    }

    // Resolve the reactor's weapon EXACTLY as dispatch_fire does (`ganger → Wields →
    // the weapon entity`, mounted-first via the ONE shared preference rule — GTW-660)
    // — the single-shot spec + Magazine + Handedness the can_fire gate + the
    // FireRequested need. A reactor wielding no weapon (or whose weapon entity is
    // missing) cannot react — fail closed.
    let (mode, live_magazine, handedness, weapon_entity) =
        reactor_weapon(reactor.entity, wields, weapons, probes)?;
    // GTW-646: the WORKING magazine — rounds net of the shots already committed from
    // this weapon this pass, so the ammo gate below matches the dispatcher's own.
    let magazine = ledger.magazine_of(weapon_entity, live_magazine);
    let fire_cost = mode_tu_cost(&mode, &reactor.tu_max, &reactor.aiming, tuning);

    // C2: the per-pair LOS + arc + fire gates — REUSED verbatim (faction-agnostic),
    // so a fired interrupt is dispatcher-accepted.
    let observer = Observer {
        position:         &reactor.position,
        stance:           &reactor.stance,
        facing:           &facing_now,
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
        return None;
    }
    // can_fire (the shared fire guard) — alive, affordable, loaded, in-bounds. The TU
    // pool and magazine are the GTW-646 working values, so affordability here matches
    // the dispatcher's own gate later this tick.
    let fire_actor = FireActor {
        life: &reactor.life,
        tu: &tu_now,
        tu_max: &reactor.tu_max,
        aiming: &reactor.aiming,
        magazine: &magazine,
        handedness,
        hands_available: HandsAvailable::default(),
    };
    if !*can_fire(&fire_actor, &mode, actor_cell, actor_level, tuning) {
        return None;
    }
    // decide_fire_arc (the SHARED arc verdict `dispatch_fire` matches) — the reactor
    // turns-to-fire if it can afford turn + shot, else the interrupt would be rejected
    // (so don't emit it). Taking the FULL verdict (not the `can_engage` boolean over
    // the same function) also yields the exact turn cost + post-turn facing the
    // dispatcher will apply — the GTW-646 commit below folds them into the ledger.
    let (facing_after, turn_cost) = match decide_fire_arc(
        *facing_now,
        reactor.position.cell(),
        actor_cell,
        tu_now,
        fire_cost,
        tuning,
    ) {
        FireArcDecision::Reject => return None,
        FireArcDecision::FireInArc => (*facing_now, Tu::new(0)),
        FireArcDecision::TurnThenFire { facing, turn_cost } => (facing, turn_cost),
    };

    // C3: the opposed check — score both sides, derive the clamped probability, roll
    // ONE seeded draw. The watcher is the reactor, the mover is the actor. The watcher
    // term reads the WORKING TU (what is actually left to fund this interrupt).
    let watcher_score = reaction_score(reactor.reactions, tu_now, reactor.tu_max);
    let mover_score = reaction_score(actor.reactions, actor.tu, actor.tu_max);
    let probability = interrupt_probability(watcher_score, mover_score, &tuning.reaction);
    if !*rolls_interrupt(probability, rng) {
        return None;
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
    //       The spend's ONE owner stays here (GTW-646): the gates above already read
    //       the pass ledger, so an emitted interrupt is dispatcher-affordable by
    //       construction — the count can never precede a shot that will not fire.
    if let Ok(mut counter) = used.get_mut(reactor.entity) {
        counter.increment();
    }
    //  (iv) predict the post-dispatch reactor state from the SAME sources the
    //       dispatcher runs (the arc verdict's turn cost + facing, the shared mode
    //       charge, the fire() round spend) and hand it back for the pass ledger.
    Some(InterruptCommit::predict(
        reactor.entity,
        weapon_entity,
        tu_now,
        Tu::new((*turn_cost).saturating_add(*fire_cost)),
        Facing::new(facing_after),
        clamp_burst(mode.shots, &magazine),
        magazine,
    ))
}

/// Resolve a reactor's single-shot fire spec + magazine + handedness — AND the weapon
/// entity they live on (the GTW-646 ledger's magazine key) — through
/// `ganger → Wields → the weapon entity`: the EXACT resolution
/// [`dispatch_fire`](crate::acts::dispatch_fire) runs, via the ONE shared preference
/// rule [`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon) (mounted-first,
/// melee-excluded — GTW-660), so the interrupt is GATED on — and fires — the same
/// weapon the dispatcher will resolve (GTW-468 C9 — reuse, don't reimplement): a
/// reactor MANNING an emplacement is gated on the mounted gun's mode / TU / magazine,
/// never its carried gun's.
///
/// `None` when the reactor wields no ranged weapon or its weapon entity is missing from
/// the weapon query — the reactor then cannot react (fail closed).
pub(super) fn reactor_weapon(
    reactor: Entity,
    wields: &WieldsQuery,
    weapons: &Query<(&Magazine, &FireMode, &Handedness)>,
    probes: &WeaponProbes,
) -> Option<(crate::weapon::FireModeSpec, Magazine, Handedness, Entity)> {
    // GTW-505 C5 / GTW-543 / GTW-660: the shared mounted-first, melee-excluded
    // resolution — the interrupt fires the mount while manning, else the carried gun,
    // never the melee weapon.
    let weapon_entity = wields.get(reactor).ok().and_then(|w| {
        w.firing_weapon(
            |entity| probes.mounted.get(entity).is_ok(),
            |entity| probes.melee.get(entity).is_ok(),
        )
    })?;
    let (magazine, fire_mode, handedness) = weapons.get(weapon_entity).ok()?;
    Some((fire_mode.single(), *magazine, *handedness, weapon_entity))
}
