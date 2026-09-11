use super::support::*;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    magazine::{LoadedRounds, Magazine, can_fire, in_bounds, mode_tu_cost},
    metric::{Cell, CellUnit, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, GridExtent},
    tuning::CombatTuning,
    weapon::MagazineSize,
};

#[test]
fn can_fire_true_when_all_guards_pass() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.2, 1);

    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    assert!(
        *can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "an alive, affordable, loaded shooter at an in-bounds target can fire"
    );
}

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

#[test]
fn can_fire_false_when_tu_short_of_mode_charge() {
    let tuning = CombatTuning::default();
    let size = MagazineSize::new(30);
    let m = mode(0.5, 1);

    let life = LifeState::Alive;
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);

    let charge = mode_tu_cost(&m, &tu_max, &aiming, &tuning);
    assert!(*charge > 0, "the mode charge must be positive for the test");

    let short = Tu::new(charge.saturating_sub(1));
    let a_short = actor(&life, &short, &tu_max, &aiming, &magazine);
    assert!(
        !*can_fire(&a_short, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a pool one below the mode charge cannot fire"
    );

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

#[test]
fn in_bounds_uses_grid_width_height_and_max_levels() {
    let w = *CellUnit::from(GridExtent::new(GRID_WIDTH));
    let h = *CellUnit::from(GridExtent::new(GRID_HEIGHT));

    assert!(
        *in_bounds(Cell::new(0, 0), Level::new(0)),
        "the origin cell on storey 0 is in-bounds"
    );
    assert!(
        *in_bounds(Cell::new(w - 1, h - 1), Level::new(MAX_LEVELS - 1)),
        "the top corner (GRID_WIDTH-1, GRID_HEIGHT-1, MAX_LEVELS-1) is in-bounds"
    );

    assert!(
        !*in_bounds(Cell::new(w, 0), Level::new(0)),
        "x == GRID_WIDTH is out of bounds"
    );
    assert!(
        !*in_bounds(Cell::new(0, h), Level::new(0)),
        "y == GRID_HEIGHT is out of bounds"
    );
    assert!(
        !*in_bounds(Cell::new(0, 0), Level::new(MAX_LEVELS)),
        "level == MAX_LEVELS is out of bounds"
    );
    assert!(
        !*in_bounds(Cell::new(-1, 0), Level::new(0)),
        "negative x is out of bounds"
    );
    assert!(
        !*in_bounds(Cell::new(0, -1), Level::new(0)),
        "negative y is out of bounds"
    );
}

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

    assert!(
        *can_fire(&a, &m, Cell::new(20, 20), Level::new(2), &tuning),
        "an alive, affordable, loaded, in-bounds shooter passes can_fire with no LOS input"
    );
}
