use super::support::{
    Actor, CombatTuning, DownedTarget, Faction, GROUND, Level, LifeState, can_stabilize, pos,
    run_stabilize, stabilize_pass,
};


#[test]
fn can_stabilize_true_for_the_canonical_ally_setup() {
    let (a, t) = stabilize_pass();
    assert!(
        *can_stabilize(&a, &t),
        "an Alive 8-adjacent same-faction actor stabilizes a Downed, bleeding-out target",
    );
}

#[test]
fn can_stabilize_sweep_each_guard_flips_it_false() {
    let (a, t) = stabilize_pass();
    let far = DownedTarget {
        pos: pos(13, 10, GROUND),
        ..t
    };
    assert!(!*can_stabilize(&a, &far), "non-adjacent must fail");

    let other_storey = DownedTarget {
        pos: pos(11, 10, Level::new(1)),
        ..t
    };
    assert!(
        !*can_stabilize(&a, &other_storey),
        "a same-x/y target one storey up is NOT adjacent — must fail",
    );

    let downed_actor = Actor {
        life: LifeState::Downed,
        ..a
    };
    assert!(
        !*can_stabilize(&downed_actor, &t),
        "a non-Alive (Downed) actor cannot stabilize",
    );
    let dead_actor = Actor {
        life: LifeState::Dead,
        ..a
    };
    assert!(
        !*can_stabilize(&dead_actor, &t),
        "a Dead actor cannot stabilize",
    );

    let alive_target = DownedTarget {
        life: LifeState::Alive,
        ..t
    };
    assert!(
        !*can_stabilize(&a, &alive_target),
        "an Alive target is not a stabilize subject",
    );

    let already = DownedTarget {
        bleeding_out: None,
        ..t
    };
    assert!(
        !*can_stabilize(&a, &already),
        "a target that is not bleeding out (already stabilized) must fail (no re-dress)",
    );

    let enemy = DownedTarget {
        faction: Faction::new(2),
        ..t
    };
    assert!(
        !*can_stabilize(&a, &enemy),
        "a cross-faction (enemy) actor cannot stabilize",
    );
}

#[test]
fn can_stabilize_rejects_a_target_that_is_not_bleeding_out() {
    let (a, mut t) = stabilize_pass();
    t.bleeding_out = None;
    assert!(
        !*can_stabilize(&a, &t),
        "a not-bleeding-out (stabilized) target is rejected — re-stabilize is a no-op",
    );
}


#[test]
fn stabilize_downed_removes_the_condition_and_never_touches_life() {
    let (a, t) = stabilize_pass();
    let tuning = CombatTuning::default();

    let (acted, marker_survived) = run_stabilize(&a, &t, &tuning);

    assert!(acted.is_some(), "the canonical setup must act");
    assert!(
        !marker_survived,
        "stabilize_downed must REMOVE the BleedingOut condition (the clock halts)",
    );
    assert_eq!(
        t.life,
        LifeState::Downed,
        "a stabilized ganger remains Downed (the verb never touches LifeState)",
    );
}


#[test]
fn stabilize_act_iff_predicate_both_directions() {
    let tuning = CombatTuning::default();

    let (a, t) = stabilize_pass();
    assert!(*can_stabilize(&a, &t));
    let (acted, marker_survived) = run_stabilize(&a, &t, &tuning);
    assert!(acted.is_some(), "predicate true ⇒ act fires");
    assert!(!marker_survived, "predicate true ⇒ condition removed");

    let far = DownedTarget {
        pos: pos(20, 20, GROUND),
        ..t
    };
    assert!(!*can_stabilize(&a, &far));
    let (acted2, marker_survived2) = run_stabilize(&a, &far, &tuning);
    assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
    assert!(marker_survived2, "predicate false ⇒ condition untouched");
}
