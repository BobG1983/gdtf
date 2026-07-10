//! GTW-443 — the hand-count clause of `can_fire` (handedness x `hands_available`).

use super::support::*;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    magazine::{LoadedRounds, Magazine, can_fire},
    metric::{Cell, Level},
    tuning::CombatTuning,
    weapon::{Handedness, MagazineSize},
};

// ── GTW-443: the can_fire hand-count clause (C4 / C5 / C6 / C9) ───────────────

#[test]
fn two_handed_weapon_refused_below_two_hands() {
    // C4 (guard half): a TwoHanded weapon at 1 hand fails can_fire even when alive,
    // affordable, loaded, and in-bounds. (The fire()/no-mutation half is C4's act test.)
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    for hands in [HandsAvailable::new(1), HandsAvailable::new(0)] {
        let a = hand_actor(
            &life,
            &tu,
            &tu_max,
            &aiming,
            &magazine,
            Handedness::TwoHanded,
            hands,
        );
        assert!(
            !*can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "a TwoHanded weapon must be refused at {hands:?} (needs two hands)"
        );
    }
}

#[test]
fn one_handed_weapon_usable_at_one_hand() {
    // C5 (guard half): a OneHanded weapon stays usable at 1 hand (a one-armed ganger
    // keeps a pistol). It only fails at 0 hands.
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    let one_hand = hand_actor(
        &life,
        &tu,
        &tu_max,
        &aiming,
        &magazine,
        Handedness::OneHanded,
        HandsAvailable::new(1),
    );
    assert!(
        *can_fire(&one_hand, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a OneHanded weapon stays usable with one working hand"
    );
    let no_hands = hand_actor(
        &life,
        &tu,
        &tu_max,
        &aiming,
        &magazine,
        Handedness::OneHanded,
        HandsAvailable::new(0),
    );
    assert!(
        !*can_fire(&no_hands, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "even a OneHanded weapon needs at least one hand"
    );
}

#[test]
fn two_handed_weapon_fires_at_two_hands() {
    // C6: the gate is CONDITIONAL, not a blanket 2H ban — a TwoHanded weapon fires
    // normally at the uninjured two-hands default.
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    let a = hand_actor(
        &life,
        &tu,
        &tu_max,
        &aiming,
        &magazine,
        Handedness::TwoHanded,
        HandsAvailable::new(2),
    );
    assert!(
        *can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a TwoHanded weapon fires normally at two hands"
    );
}

#[test]
fn uninjured_default_actor_fires_both_handedness() {
    // C9 (guard half): the default `actor` helper (OneHanded, two hands) passes — and a
    // TwoHanded actor at the same default two hands also passes — so the hand-count clause
    // never spuriously gates an uninjured shooter.
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    let default_actor = actor(&life, &tu, &tu_max, &aiming, &magazine);
    assert!(
        *can_fire(
            &default_actor,
            &m,
            Cell::new(10, 10),
            Level::new(0),
            &tuning
        ),
        "an uninjured (OneHanded, two-hands) actor passes the hand-count clause"
    );
}
