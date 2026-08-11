use super::support::{
    CombatTuning, GROUND, LifeState, actor, can_execute, can_stabilize, execute_downed, funded,
    pos, run_stabilize, target,
};

#[test]
fn faction_gate_enemy_cannot_stabilize() {
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 2, true);
    let tuning = CombatTuning::default();

    assert!(
        !*can_stabilize(&a, &t, &funded(), &tuning),
        "an enemy (cross-faction) cannot stabilize — predicate false",
    );

    let (acted, marker_survived) = run_stabilize(&a, &t, funded(), &tuning);
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
    let a = actor(pos(10, 10, GROUND), LifeState::Alive, 1);
    let t = target(pos(11, 10, GROUND), LifeState::Downed, 1, true);
    let tuning = CombatTuning::default();

    assert!(
        !*can_execute(&a, &t, &funded(), &tuning),
        "an ally (same-faction) cannot execute — predicate false",
    );

    let mut life = LifeState::Downed;
    let acted = execute_downed(&a, &t, &mut life, &funded(), &tuning);
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
