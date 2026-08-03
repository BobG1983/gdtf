use bevy::prelude::{Entity, Query, With};

use super::{
    declared::{InterruptDeclared, InterruptSignals},
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
    signals: &mut InterruptSignals,
) -> Option<InterruptCommit> {
    let actor_cell = actor.position.cell();
    let actor_level = actor.position.level();

    let tu_now = ledger.tu_of(reactor.entity, reactor.tu);
    let facing_now = ledger.facing_of(reactor.entity, reactor.facing);

    if !*reactor.life.is_active() || *tu_now == 0 {
        return None;
    }
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

    let (mode, live_magazine, handedness, weapon_entity) =
        reactor_weapon(reactor.entity, wields, weapons, probes)?;
    let magazine = ledger.magazine_of(weapon_entity, live_magazine);
    let fire_cost = mode_tu_cost(&mode, &reactor.tu_max, &reactor.aiming, tuning);

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

    let watcher_score = reaction_score(reactor.reactions, tu_now, reactor.tu_max);
    let mover_score = reaction_score(actor.reactions, actor.tu, actor.tu_max);
    let probability = interrupt_probability(watcher_score, mover_score, &tuning.reaction);
    if !*rolls_interrupt(probability, rng) {
        return None;
    }

    signals.fire.write(FireRequested::new(
        reactor.entity,
        mode,
        actor_cell,
        actor_level,
    ));
    signals.halt.write(ReactionShotFired::new(actor.entity));
    signals
        .declared
        .write(InterruptDeclared::new(reactor.entity, actor.entity));
    if let Ok(mut counter) = used.get_mut(reactor.entity) {
        counter.increment();
    }
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

pub(super) fn reactor_weapon(
    reactor: Entity,
    wields: &WieldsQuery,
    weapons: &Query<(&Magazine, &FireMode, &Handedness)>,
    probes: &WeaponProbes,
) -> Option<(crate::weapon::FireModeSpec, Magazine, Handedness, Entity)> {
    let weapon_entity = wields.get(reactor).ok().and_then(|w| {
        w.firing_weapon(
            |entity| probes.mounted.get(entity).is_ok(),
            |entity| probes.melee.get(entity).is_ok(),
        )
    })?;
    let (magazine, fire_mode, handedness) = weapons.get(weapon_entity).ok()?;
    Some((fire_mode.single(), *magazine, *handedness, weapon_entity))
}
