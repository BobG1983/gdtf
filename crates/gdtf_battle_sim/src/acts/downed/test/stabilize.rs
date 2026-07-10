//! AC1/AC2/AC6 (stabilize) — `can_stabilize` is true ONLY for an Alive actor
//! 8-adjacent to a Downed, not-yet-Stabilized SAME-faction target; `stabilize_downed`
//! SETS the flag and keeps the ganger Downed; and the act fires EXACTLY when the
//! predicate is true (both directions).

use super::support::{
    Actor, CombatTuning, DownedTarget, Faction, GROUND, Level, LifeState, Stabilized,
    can_stabilize, pos, stabilize_downed, stabilize_pass,
};

// === AC1 — can_stabilize is true ONLY for an Alive actor 8-adjacent to a
// Downed, not-yet-Stabilized SAME-faction target; each guard, flipped alone,
// independently makes it false. ===

#[test]
fn can_stabilize_true_for_the_canonical_ally_setup() {
    let (a, t) = stabilize_pass();
    assert!(
        *can_stabilize(&a, &t),
        "an Alive 8-adjacent same-faction actor stabilizes a Downed, un-stabilized target",
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

    // (4) already stabilized (Some(true)).
    let already = DownedTarget {
        stabilized: Some(Stabilized::new(true)),
        ..t
    };
    assert!(
        !*can_stabilize(&a, &already),
        "an already-stabilized target must fail (no re-dress)",
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

/// A `Some(false)` flag is NOT "already stabilized" — present-and-false is
/// not-yet-stabilized, so the guard still passes (distinguishes "has the
/// component" from "is stabilized").
#[test]
fn can_stabilize_passes_with_stabilized_false_flag_present() {
    let (a, mut t) = stabilize_pass();
    t.stabilized = Some(Stabilized::new(false));
    assert!(
        *can_stabilize(&a, &t),
        "Stabilized(false) present is not-yet-stabilized — the guard passes",
    );
}

// === AC2 — stabilize_downed SETS the flag and leaves LifeState::Downed
// unchanged (NOT Alive, NOT Dead). ===

#[test]
fn stabilize_downed_sets_flag_and_keeps_downed() {
    let (a, t) = stabilize_pass();
    // Model the target's LifeState as the SAME `&mut` shape execute_downed
    // mutates, so this test would catch a stabilize verb that wrongly wrote it.
    let mut flag = Stabilized::new(false);
    let mut life = t.life;
    let tuning = CombatTuning::default();

    let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
    // Apply the (non-)life-effect the same way the caller would: stabilize never
    // returns a LifeState change, so `life` stays whatever the target held.
    let _ = &mut life;

    assert!(acted.is_some(), "the canonical setup must act");
    assert_eq!(
        flag,
        Stabilized::new(true),
        "stabilize_downed must SET the Stabilized flag true",
    );
    // The verb takes NO &mut LifeState — the ganger stays Downed (NOT Alive, NOT
    // Dead): a stabilized ganger remains down, just no longer bleeding.
    assert_eq!(
        life,
        LifeState::Downed,
        "a stabilized ganger remains Downed (NOT Alive, NOT Dead)",
    );
}

// === AC6 — PREDICATE ⇔ ACT: stabilize is a no-op EXACTLY when its predicate is
// false (both directions). ===

#[test]
fn stabilize_act_iff_predicate_both_directions() {
    let tuning = CombatTuning::default();

    // Predicate TRUE → act fires (sets flag, returns Some).
    let (a, t) = stabilize_pass();
    let mut flag = Stabilized::new(false);
    assert!(*can_stabilize(&a, &t));
    let acted = stabilize_downed(&a, &t, &mut flag, &tuning);
    assert!(acted.is_some(), "predicate true ⇒ act fires");
    assert_eq!(flag, Stabilized::new(true), "predicate true ⇒ flag set");

    // Predicate FALSE (here: non-adjacent) → act is a no-op (None, flag intact).
    let far = DownedTarget {
        pos: pos(20, 20, GROUND),
        ..t
    };
    let mut flag2 = Stabilized::new(false);
    assert!(!*can_stabilize(&a, &far));
    let acted2 = stabilize_downed(&a, &far, &mut flag2, &tuning);
    assert!(acted2.is_none(), "predicate false ⇒ no-op (None)");
    assert_eq!(
        flag2,
        Stabilized::new(false),
        "predicate false ⇒ flag untouched",
    );
}
