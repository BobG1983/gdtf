use super::support::{
    Actor, CombatTuning, DownedTarget, Faction, GROUND, Level, LifeState, can_execute,
    execute_downed, execute_pass, execute_tu_cost, funded, one_short, pos,
};

#[test]
fn can_execute_true_for_the_canonical_enemy_setup() {
    let (a, t) = execute_pass();
    let tuning = CombatTuning::default();
    assert!(
        *can_execute(&a, &t, &funded(), &tuning),
        "an Alive 8-adjacent opposing-faction actor executes a Downed target",
    );
}

#[test]
fn can_execute_sweep_each_guard_flips_it_false() {
    let (a, t) = execute_pass();
    let tuning = CombatTuning::default();
    let pool = funded();

    let far = DownedTarget {
        pos: pos(13, 10, GROUND),
        ..t
    };
    assert!(
        !*can_execute(&a, &far, &pool, &tuning),
        "non-adjacent must fail"
    );

    let other_storey = DownedTarget {
        pos: pos(11, 10, Level::new(1)),
        ..t
    };
    assert!(
        !*can_execute(&a, &other_storey, &pool, &tuning),
        "a same-x/y target one storey up is NOT adjacent — must fail",
    );

    let downed_actor = Actor {
        life: LifeState::Downed,
        ..a
    };
    assert!(
        !*can_execute(&downed_actor, &t, &pool, &tuning),
        "a Downed actor cannot execute"
    );

    let alive_target = DownedTarget {
        life: LifeState::Alive,
        ..t
    };
    assert!(
        !*can_execute(&a, &alive_target, &pool, &tuning),
        "an Alive target is not an execute subject",
    );

    let ally = DownedTarget {
        faction: Faction::new(1),
        ..t
    };
    assert!(
        !*can_execute(&a, &ally, &pool, &tuning),
        "a same-faction (ally) actor cannot execute",
    );

    let cost = execute_tu_cost(&tuning);
    assert!(
        *cost > 0,
        "execute must be priced for a short pool to exist"
    );
    assert!(
        !*can_execute(&a, &t, &one_short(cost), &tuning),
        "a pool one short of execute_tu_cost cannot afford the act",
    );
}

#[test]
fn can_execute_ignores_bleeding_out_condition() {
    let (a, mut t) = execute_pass();
    let tuning = CombatTuning::default();
    t.bleeding_out = None;
    assert!(
        *can_execute(&a, &t, &funded(), &tuning),
        "a stabilized Downed enemy can still be executed (the condition does not gate execute)",
    );
}

#[test]
fn execute_downed_kills_the_target() {
    let (a, t) = execute_pass();
    let mut life = LifeState::Downed;
    let tuning = CombatTuning::default();

    let acted = execute_downed(&a, &t, &mut life, &funded(), &tuning);

    assert!(acted.is_some(), "the canonical setup must act");
    assert_eq!(
        life,
        LifeState::Dead,
        "execute_downed must transition the target outright to Dead",
    );
}

#[test]
fn execute_act_iff_predicate_both_directions() {
    let tuning = CombatTuning::default();
    let pool = funded();

    let (a, t) = execute_pass();
    let mut life = LifeState::Downed;
    assert!(*can_execute(&a, &t, &pool, &tuning));
    let acted = execute_downed(&a, &t, &mut life, &pool, &tuning);
    assert!(acted.is_some(), "predicate true ⇒ act fires");
    assert_eq!(life, LifeState::Dead, "predicate true ⇒ target Dead");

    let far = DownedTarget {
        pos: pos(20, 20, GROUND),
        ..t
    };
    let mut life2 = LifeState::Downed;
    assert!(!*can_execute(&a, &far, &pool, &tuning));
    let acted2 = execute_downed(&a, &far, &mut life2, &pool, &tuning);
    assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
    assert_eq!(
        life2,
        LifeState::Downed,
        "predicate false ⇒ target unchanged (still Downed)",
    );
}
