//! Evaluate one actor/reactor pair for opportunity fire.

use bevy::prelude::Entity;

use super::{
    declared::{InterruptDeclared, InterruptSignals},
    ledger::InterruptCommit,
    params::{ReactionGrids, ReactorArms, ReactorEligibility, ReactorShot},
    snapshot::{ReactionPair, ReactionPass, row_cell_level},
};
use crate::{
    acts::{
        FireArcDecision, FireRequested, decide_fire_arc, fire_arc_tu_cost,
        movement::ReactionShotFired,
    },
    ganger::{Facing, LifeState, Tu},
    injuries::HandsAvailable,
    los::{Observer, PeekOffset, Target, can_see},
    magazine::{FireActor, can_fire, clamp_burst, mode_tu_cost},
    metric::Cell,
    rng::ReactionRng,
    tuning::{CombatTuning, interrupt_probability, may_interrupt, reaction_score, rolls_interrupt},
};

/// Try to interrupt the pair's actor with its reactor; a commit means fire was declared.
pub(super) fn try_reaction(
    pair: ReactionPair<'_>,
    pass: &ReactionPass<'_>,
    arms: &ReactorArms,
    eligibility: &mut ReactorEligibility,
    grids: &ReactionGrids,
    rng: &mut ReactionRng,
    signals: &mut InterruptSignals,
) -> Option<InterruptCommit> {
    let ReactionPair { actor, reactor } = pair;
    let tuning = grids.tuning();
    let actor_cell = actor.position.cell();
    let actor_level = actor.position.level();

    let tu_now = pass.tu_of(reactor.entity, reactor.tu);
    let facing_now = pass.facing_of(reactor.entity, reactor.facing);

    if !*reactor.life.is_active() || *tu_now == 0 {
        return None;
    }
    if *eligibility.suppressed(reactor.entity) {
        return None;
    }
    let used_now = eligibility.used_by(reactor.entity);
    if !*may_interrupt(used_now, reactor.reactions, &tuning.reaction) {
        return None;
    }

    let ReactorShot {
        mode,
        magazine: live_magazine,
        handedness,
        weapon,
    } = arms.shot(reactor.entity)?;
    let magazine = pass.magazine_of(weapon, live_magazine);
    let fire_cost = mode_tu_cost(&mode, &reactor.tu_max, &reactor.aiming, tuning);

    let observer = Observer {
        position:         &reactor.position,
        stance:           &reactor.stance,
        facing:           &facing_now,
        stair_eye_offset: grids
            .occupancy()
            .stair_eye_offset_at(&row_cell_level(&reactor.position)),
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
        grids.march(),
        tuning,
        |entity: Entity| pass.life_of(entity) == LifeState::Dead,
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
    let (facing_after, spend) = shot_pose(
        facing_now,
        reactor.position.cell(),
        actor_cell,
        tu_now,
        fire_cost,
        tuning,
    )?;
    let watcher_score = reaction_score(reactor.reactions, tu_now, reactor.tu_max);
    let mover_score = reaction_score(actor.reactions, actor.tu, actor.tu_max);
    let probability = interrupt_probability(watcher_score, mover_score, &tuning.reaction);
    if !*rolls_interrupt(probability, rng) {
        return None;
    }

    let commit = InterruptCommit::predict(
        reactor.entity,
        weapon,
        tu_now,
        spend,
        facing_after,
        clamp_burst(mode.shots, &magazine),
        magazine,
    )?;

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
    eligibility.record_use(reactor.entity);
    Some(commit)
}

// Where the reactor ends up facing, and what turning and shooting takes out of its pool.
fn shot_pose(
    facing_now: Facing,
    reactor_cell: Cell,
    actor_cell: Cell,
    tu_now: Tu,
    fire_cost: Tu,
    tuning: &CombatTuning,
) -> Option<(Facing, Tu)> {
    let facing_after = match decide_fire_arc(
        *facing_now,
        reactor_cell,
        actor_cell,
        tu_now,
        fire_cost,
        tuning,
    ) {
        FireArcDecision::Reject => return None,
        FireArcDecision::FireInArc => *facing_now,
        FireArcDecision::TurnThenFire { facing, .. } => facing,
    };
    Some((
        Facing::new(facing_after),
        fire_arc_tu_cost(*facing_now, reactor_cell, actor_cell, fire_cost, tuning),
    ))
}
