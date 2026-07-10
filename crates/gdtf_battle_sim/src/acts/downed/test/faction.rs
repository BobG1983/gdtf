//! AC5 — the FACTION GATE: an enemy cannot stabilize; an ally cannot execute, for
//! both the predicate AND the act (no-op).

use super::support::{
    CombatTuning, GROUND, LifeState, Stabilized, actor, can_execute, can_stabilize, execute_downed,
    pos, stabilize_downed, target,
};

#[test]
fn faction_gate_enemy_cannot_stabilize() {
    // An Alive, 8-adjacent actor and a Downed target of a DIFFERENT faction.
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, None);

    // Predicate: false.
    assert!(
        !*can_stabilize(&a, &t),
        "an enemy (cross-faction) cannot stabilize — predicate false",
    );

    // Act: no-op (flag untouched, returns None).
    let mut flag = Stabilized::new(false);
    let tuning = CombatTuning::default();
    let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
    assert!(
        acted.is_none(),
        "an enemy's stabilize_downed is a no-op (None)"
    );
    assert_eq!(
        flag,
        Stabilized::new(false),
        "an enemy's stabilize_downed must NOT set the flag",
    );
}

#[test]
fn faction_gate_ally_cannot_execute() {
    // An Alive, 8-adjacent actor and a Downed target of the SAME faction.
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, None);

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
