//! AC3/AC4/AC6 (execute) — `can_execute` is true ONLY for an Alive actor 8-adjacent
//! to a Downed OPPOSING-faction target (and ignores the `BleedingOut` condition);
//! `execute_downed` transitions the target to Dead; and the act fires EXACTLY when
//! the predicate is true (both directions).

use super::support::{
    Actor, CombatTuning, DownedTarget, Faction, GROUND, Level, LifeState, can_execute,
    execute_downed, execute_pass, pos,
};

// === AC3 — can_execute is true ONLY for an Alive actor 8-adjacent to a Downed
// OPPOSING-faction target; parallel guard sweep. ===

#[test]
fn can_execute_true_for_the_canonical_enemy_setup() {
    let (a, t) = execute_pass();
    assert!(
        *can_execute(&a, &t),
        "an Alive 8-adjacent opposing-faction actor executes a Downed target",
    );
}

#[test]
fn can_execute_sweep_each_guard_flips_it_false() {
    let (a, t) = execute_pass();

    // (1) NOT adjacent.
    let far = DownedTarget {
        pos: pos(13, 10, GROUND),
        ..t
    };
    assert!(!*can_execute(&a, &far), "non-adjacent must fail");

    // (1b) NOT adjacent — different storey.
    let other_storey = DownedTarget {
        pos: pos(11, 10, Level::new(1)),
        ..t
    };
    assert!(
        !*can_execute(&a, &other_storey),
        "a same-x/y target one storey up is NOT adjacent — must fail",
    );

    // (2) actor not Alive.
    let downed_actor = Actor {
        life: LifeState::Downed,
        ..a
    };
    assert!(
        !*can_execute(&downed_actor, &t),
        "a Downed actor cannot execute"
    );

    // (3) target not Downed.
    let alive_target = DownedTarget {
        life: LifeState::Alive,
        ..t
    };
    assert!(
        !*can_execute(&a, &alive_target),
        "an Alive target is not an execute subject",
    );

    // (4) same-faction (an ALLY cannot execute).
    let ally = DownedTarget {
        faction: Faction::new(1),
        ..t
    };
    assert!(
        !*can_execute(&a, &ally),
        "a same-faction (ally) actor cannot execute",
    );
}

/// A stabilized Downed enemy (no `BleedingOut` condition) can STILL be executed — the
/// condition does not gate execute (only stabilize is blocked by its absence).
#[test]
fn can_execute_ignores_bleeding_out_condition() {
    let (a, mut t) = execute_pass();
    t.bleeding_out = None;
    assert!(
        *can_execute(&a, &t),
        "a stabilized Downed enemy can still be executed (the condition does not gate execute)",
    );
}

// === AC4 — execute_downed transitions the target to LifeState::Dead. ===

#[test]
fn execute_downed_kills_the_target() {
    let (a, t) = execute_pass();
    let mut life = LifeState::Downed;
    let tuning = CombatTuning::default();

    let acted = execute_downed(&a, &t, &mut life, &tuning);

    assert!(acted.is_some(), "the canonical setup must act");
    assert_eq!(
        life,
        LifeState::Dead,
        "execute_downed must transition the target outright to Dead",
    );
}

// === AC6 — PREDICATE ⇔ ACT: execute is a no-op EXACTLY when its predicate is
// false (both directions). ===

#[test]
fn execute_act_iff_predicate_both_directions() {
    let tuning = CombatTuning::default();

    // Predicate TRUE → act fires (kills, returns Some).
    let (a, t) = execute_pass();
    let mut life = LifeState::Downed;
    assert!(*can_execute(&a, &t));
    let acted = execute_downed(&a, &t, &mut life, &tuning);
    assert!(acted.is_some(), "predicate true ⇒ act fires");
    assert_eq!(life, LifeState::Dead, "predicate true ⇒ target Dead");

    // Predicate FALSE (here: non-adjacent) → act is a no-op (None, life intact).
    let far = DownedTarget {
        pos: pos(20, 20, GROUND),
        ..t
    };
    let mut life2 = LifeState::Downed;
    assert!(!*can_execute(&a, &far));
    let acted2 = execute_downed(&a, &far, &mut life2, &tuning);
    assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
    assert_eq!(
        life2,
        LifeState::Downed,
        "predicate false ⇒ target unchanged (still Downed)",
    );
}
