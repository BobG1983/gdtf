//! The `can_fire` guard clauses AC3-AC8, incl. the `in_bounds` predicate and the
//! no-LOS boundary (mirrors `guard.rs`).

use super::support::*;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    magazine::{LoadedRounds, Magazine, can_fire, in_bounds, mode_tu_cost},
    metric::{Cell, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    tuning::CombatTuning,
    weapon::MagazineSize,
};

// AC3 — can_fire is TRUE when all guards pass: Alive + affords the mode TU +
// >=1 round + in-bounds target.

#[test]
fn can_fire_true_when_all_guards_pass() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);

    let life = LifeState::Alive;
    let tu = Tu::new(255); // amply affords any charge
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    assert!(
        *can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "an alive, affordable, loaded shooter at an in-bounds target can fire"
    );
}

// AC4 — can_fire is FALSE when the shooter is not Alive (Downed/Dead), all
// else affordable.

#[test]
fn can_fire_false_when_not_alive() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);

    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    for life in [LifeState::Downed, LifeState::Dead] {
        let a = actor(&life, &tu, &tu_max, &aiming, &magazine);
        assert!(
            !*can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "a {life:?} shooter cannot fire even when everything else is affordable"
        );
    }
}

// AC5 — can_fire is FALSE when Tu is short of the mode charge; and the AIMING
// case requires strictly MORE TU than hip-fire (RELATION via AimTuPremium,
// never the literal 1.5).

#[test]
fn can_fire_false_when_tu_short_of_mode_charge() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.5, 1);

    let life = LifeState::Alive;
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    // The exact hip-fire charge for this mode.
    let charge = mode_tu_cost(&m, &tu_max, &aiming, &tuning);
    assert!(*charge > 0, "the mode charge must be positive for the test");

    // One TU short of the charge → cannot afford → false.
    let short = Tu::new(charge.saturating_sub(1));
    let a_short = actor(&life, &short, &tu_max, &aiming, &magazine);
    assert!(
        !*can_fire(&a_short, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a pool one below the mode charge cannot fire"
    );

    // Exactly the charge → can afford → true (equality affords).
    let exact = Tu::new(*charge);
    let a_exact = actor(&life, &exact, &tu_max, &aiming, &magazine);
    assert!(
        *can_fire(&a_exact, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a pool exactly the mode charge can fire"
    );
}

#[test]
fn aiming_costs_strictly_more_tu_than_hip_fire() {
    let tuning = CombatTuning::default();
    // The aim premium is > 1 (the cost lever), so aiming costs strictly more.
    // We assert the RELATION, never the literal 1.5 magnitude.
    assert!(
        *tuning.cone_stability.aim_mode.tu_premium > 1.0,
        "precondition: the aim TU premium is a >1 cost multiplier"
    );

    let m = mode(0.5, 1);
    let tu_max = TuMax::new(100);

    let hip = mode_tu_cost(&m, &tu_max, &Aiming::new(false), &tuning);
    let aimed = mode_tu_cost(&m, &tu_max, &Aiming::new(true), &tuning);

    assert!(
        *aimed > *hip,
        "aiming must cost strictly more TU than hip-fire (charge {} vs {})",
        *aimed,
        *hip,
    );
}

// AC6 — can_fire is FALSE when the Magazine is empty (0 rounds), all else
// affordable.

#[test]
fn can_fire_false_when_magazine_empty() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);

    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let empty = Magazine::new(LoadedRounds::new(0), size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &empty);

    assert!(
        !*can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "an empty magazine cannot fire even when everything else is affordable"
    );
}

// AC7 — can_fire's in-bounds predicate uses GRID_WIDTH/GRID_HEIGHT (x/y) +
// MAX_LEVELS (z): an in-grid target passes; x==GRID_WIDTH, y==GRID_HEIGHT,
// level==MAX_LEVELS, and any negative axis each FAIL. Structural-constant pins
// (the coordinate-system exemption — these ARE the grid's real extents).

#[test]
fn in_bounds_uses_grid_width_height_and_max_levels() {
    // The grid extents as i32 cell coordinates — a checked conversion
    // (GRID_WIDTH/GRID_HEIGHT are usize; a raw `as i32` trips cast_possible_wrap).
    // The fallback is unreachable for the 60-cell extents but keeps the test
    // free of unwrap/expect (denied in tests too).
    let w = i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX);

    // A target well inside the grid passes.
    assert!(
        *in_bounds(Cell::new(0, 0), Level::new(0)),
        "the origin cell on storey 0 is in-bounds"
    );
    assert!(
        *in_bounds(Cell::new(w - 1, h - 1), Level::new(MAX_LEVELS - 1)),
        "the top corner (GRID_WIDTH-1, GRID_HEIGHT-1, MAX_LEVELS-1) is in-bounds"
    );

    // x == GRID_WIDTH is one past the x extent → out.
    assert!(
        !*in_bounds(Cell::new(w, 0), Level::new(0)),
        "x == GRID_WIDTH is out of bounds"
    );
    // y == GRID_HEIGHT is one past the y extent → out.
    assert!(
        !*in_bounds(Cell::new(0, h), Level::new(0)),
        "y == GRID_HEIGHT is out of bounds"
    );
    // level == MAX_LEVELS is one past the z extent → out.
    assert!(
        !*in_bounds(Cell::new(0, 0), Level::new(MAX_LEVELS)),
        "level == MAX_LEVELS is out of bounds"
    );
    // Any negative axis is out (a Cell wraps a signed IVec2).
    assert!(
        !*in_bounds(Cell::new(-1, 0), Level::new(0)),
        "negative x is out of bounds"
    );
    assert!(
        !*in_bounds(Cell::new(0, -1), Level::new(0)),
        "negative y is out of bounds"
    );
}

// AC8 — can_fire takes NO has_los/visibility input: the boundary is that LOS
// is presenter player policy (fog "never enters the shared act"). There is no
// LOS parameter to vary; this documents that an in-bounds, affordable, alive,
// loaded shooter passes can_fire's signature-level inputs alone — regardless
// of any (presenter-side) LOS state, which can_fire cannot even observe.

#[test]
fn can_fire_takes_no_los_input_and_passes_regardless() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);

    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    // can_fire's full input set is (actor, mode, cell, level, tuning) — no
    // has_los / visibility argument exists. The shooter passes on those inputs
    // alone; whether a target is "seen" is the presenter's fog gate, which the
    // model act never consults.
    assert!(
        *can_fire(&a, &m, Cell::new(20, 20), Level::new(2), &tuning),
        "an alive, affordable, loaded, in-bounds shooter passes can_fire with no LOS input"
    );
}
