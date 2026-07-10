//! AC5 — the FACTION GATE: an enemy cannot stabilize; an ally cannot execute, for
//! both the predicate AND the act (no-op).

use super::support::{
    CombatTuning, GROUND, LifeState, actor, can_execute, can_stabilize, execute_downed, pos,
    run_stabilize, target,
};

#[test]
fn faction_gate_enemy_cannot_stabilize() {
    // An Alive, 8-adjacent actor and a Downed target of a DIFFERENT faction.
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, true);

    // Predicate: false.
    assert!(
        !*can_stabilize(&a, &t),
        "an enemy (cross-faction) cannot stabilize — predicate false",
    );

    // Act: no-op (condition untouched, returns None).
    let tuning = CombatTuning::default();
    let (acted, marker_survived) = run_stabilize(&a, &t, &tuning);
    assert!(
        acted.is_none(),
        "an enemy's stabilize_downed is a no-op (None)"
    );
    assert!(
        marker_survived,
        "an enemy's stabilize_downed must NOT remove the BleedingOut condition",
    );
}

#[test]
fn faction_gate_ally_cannot_execute() {
    // An Alive, 8-adjacent actor and a Downed target of the SAME faction.
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, true);

    // Predicate: false.
    assert!(
        !*can_execute(&a, &t),
        "an ally (same-faction) cannot execute — predicate false",
    );

    // Act: no-op (life untouched, returns None).
    let mut life = LifeState::Downed;
    let tuning = CombatTuning::default();
    let acted = execute_downed(&a, &t, &mut life, &tuning);
    assert!(
        acted.is_none(),
        "an ally's execute_downed is a no-op (None)"
    );
    assert_eq!(
        life,
        LifeState::Downed,
        "an ally's execute_downed must NOT kill the target",
    );
}
