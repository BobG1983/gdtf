//! AC1/AC2/AC6 (stabilize) — `can_stabilize` is true ONLY for an Alive actor
//! 8-adjacent to a Downed, still-bleeding-out SAME-faction target; `stabilize_downed`
//! REMOVES the `BleedingOut` condition (GTW-695) and never touches `LifeState`; and the
//! act fires EXACTLY when the predicate is true (both directions).

use super::support::{
    Actor, CombatTuning, DownedTarget, Faction, GROUND, Level, LifeState, can_stabilize, pos,
    run_stabilize, stabilize_pass,
};

// === AC1 — can_stabilize is true ONLY for an Alive actor 8-adjacent to a
// Downed, still-bleeding-out SAME-faction target; each guard, flipped alone,
// independently makes it false. ===

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
    // (1) NOT adjacent (two cells away).
    let (a, t) = stabilize_pass();
    let far = DownedTarget {
        pos: pos(13, 10, GROUND),
        ..t
    };
    assert!(!*can_stabilize(&a, &far), "non-adjacent must fail");

    // (1b) NOT adjacent because a DIFFERENT level (same x/y, z+1).
    let other_storey = DownedTarget {
        pos: pos(11, 10, Level::new(1)),
        ..t
    };
    assert!(
        !*can_stabilize(&a, &other_storey),
        "a same-x/y target one storey up is NOT adjacent — must fail",
    );

    // (2) actor not Alive (Downed actor).
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

    // (3) target not Downed (Alive target).
    let alive_target = DownedTarget {
        life: LifeState::Alive,
        ..t
    };
    assert!(
        !*can_stabilize(&a, &alive_target),
        "an Alive target is not a stabilize subject",
    );

    // (4) NOT bleeding out (already stabilized — the condition is absent).
    let already = DownedTarget {
        bleeding_out: None,
        ..t
    };
    assert!(
        !*can_stabilize(&a, &already),
        "a target that is not bleeding out (already stabilized) must fail (no re-dress)",
    );

    // (5) cross-faction (an ENEMY cannot stabilize).
    let enemy = DownedTarget {
        faction: Faction::new(2),
        ..t
    };
    assert!(
        !*can_stabilize(&a, &enemy),
        "a cross-faction (enemy) actor cannot stabilize",
    );
}

/// Re-stabilizing a target that is NOT bleeding out (its condition already removed) is a
/// guarded no-op — "marker present" is the guard, so a stabilized target is rejected.
#[test]
fn can_stabilize_rejects_a_target_that_is_not_bleeding_out() {
    let (a, mut t) = stabilize_pass();
    t.bleeding_out = None;
    assert!(
        !*can_stabilize(&a, &t),
        "a not-bleeding-out (stabilized) target is rejected — re-stabilize is a no-op",
    );
}

// === AC2 — stabilize_downed REMOVES the BleedingOut condition and leaves LifeState
// untouched (the verb never writes it — the ganger stays Downed). ===

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
    // The verb takes NO &mut LifeState — the ganger stays Downed by construction (this
    // test's target.life is Downed and the verb never writes it).
    assert_eq!(
        t.life,
        LifeState::Downed,
        "a stabilized ganger remains Downed (the verb never touches LifeState)",
    );
}

// === AC6 — PREDICATE ⇔ ACT: stabilize is a no-op EXACTLY when its predicate is
// false (both directions). ===

#[test]
fn stabilize_act_iff_predicate_both_directions() {
    let tuning = CombatTuning::default();

    // Predicate TRUE → act fires (removes the condition, returns Some).
    let (a, t) = stabilize_pass();
    assert!(*can_stabilize(&a, &t));
    let (acted, marker_survived) = run_stabilize(&a, &t, &tuning);
    assert!(acted.is_some(), "predicate true ⇒ act fires");
    assert!(!marker_survived, "predicate true ⇒ condition removed");

    // Predicate FALSE (here: non-adjacent) → act is a no-op (None, condition intact).
    let far = DownedTarget {
        pos: pos(20, 20, GROUND),
        ..t
    };
    assert!(!*can_stabilize(&a, &far));
    let (acted2, marker_survived2) = run_stabilize(&a, &far, &tuning);
    assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
    assert!(marker_survived2, "predicate false ⇒ condition untouched");
}
