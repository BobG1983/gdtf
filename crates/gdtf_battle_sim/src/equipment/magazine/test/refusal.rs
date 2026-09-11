use super::support::*;
use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    magazine::{FireRefusal, LoadedRounds, Magazine, fire_refusal, mode_tu_cost},
    metric::{Cell, CellUnit, Level},
    occupancy::{GRID_WIDTH, GridExtent},
    tuning::CombatTuning,
    weapon::{Handedness, MagazineSize},
};

const ON_GRID: Cell = Cell::new(10, 10);

const GROUND: Level = Level::new(0);

// A magazine with rounds in it, so only the case that empties it can answer.
fn loaded() -> Magazine {
    Magazine::new(LoadedRounds::new(10), MagazineSize::new(30), RELOAD_TU)
}

fn off_grid() -> Cell {
    Cell::new(*CellUnit::from(GridExtent::new(GRID_WIDTH)), 10)
}

#[test]
fn a_shooter_that_is_not_alive_answers_the_not_alive_reason() {
    let tuning = CombatTuning::default();
    let m = mode(0.2, 1);
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = loaded();

    for life in [LifeState::Downed, LifeState::Dead] {
        let a = actor(&life, &tu, &tu_max, &aiming, &magazine);
        assert_eq!(
            fire_refusal(&a, &m, ON_GRID, GROUND, &tuning),
            Some(FireRefusal::NotAlive),
            "a {life:?} shooter that can afford the mode, holds rounds, aims on the grid and has \
             both hands is refused for its life state and nothing else"
        );
    }
}

#[test]
fn a_pool_one_short_of_the_mode_charge_answers_the_unaffordable_reason() {
    let tuning = CombatTuning::default();
    let m = mode(0.5, 1);
    let life = LifeState::Alive;
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = loaded();

    let charge = mode_tu_cost(&m, &tu_max, &aiming, &tuning);
    assert!(*charge > 0, "the mode charge must be positive for the test");
    let tu = Tu::new(charge.saturating_sub(1));
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    assert_eq!(
        fire_refusal(&a, &m, ON_GRID, GROUND, &tuning),
        Some(FireRefusal::Unaffordable),
        "an alive shooter holding rounds, aiming on the grid with both hands, one TU short of \
         the {charge:?} charge, is refused for the pool and nothing else"
    );
}

#[test]
fn an_empty_magazine_answers_the_magazine_empty_reason() {
    let tuning = CombatTuning::default();
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let empty = Magazine::new(LoadedRounds::new(0), MagazineSize::new(30), RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &empty);

    assert_eq!(
        fire_refusal(&a, &m, ON_GRID, GROUND, &tuning),
        Some(FireRefusal::MagazineEmpty),
        "an alive shooter that can afford the mode and aims on the grid with both hands is \
         refused for the empty magazine and nothing else"
    );
}

#[test]
fn a_target_off_the_grid_answers_the_out_of_bounds_reason() {
    let tuning = CombatTuning::default();
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = loaded();
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    assert_eq!(
        fire_refusal(&a, &m, off_grid(), GROUND, &tuning),
        Some(FireRefusal::OutOfBounds),
        "an alive shooter that can afford the mode, holds rounds and has both hands is refused \
         for the target sitting at x == GRID_WIDTH and nothing else"
    );
}

#[test]
fn a_two_handed_weapon_in_one_hand_answers_the_not_enough_hands_reason() {
    let tuning = CombatTuning::default();
    let m = mode(0.2, 1);
    let life = LifeState::Alive;
    let tu = Tu::new(255);
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let magazine = loaded();
    let a = hand_actor(
        &life,
        &tu,
        &tu_max,
        &aiming,
        &magazine,
        Handedness::TwoHanded,
        HandsAvailable::new(1),
    );

    assert_eq!(
        fire_refusal(&a, &m, ON_GRID, GROUND, &tuning),
        Some(FireRefusal::NotEnoughHands),
        "an alive shooter that can afford the mode, holds rounds and aims on the grid is refused \
         for having one hand on a two-handed weapon and nothing else"
    );
}
