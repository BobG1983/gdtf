use super::support::*;
use crate::{
    magazine::{LoadedRounds, Magazine, ReloadTu, clamp_burst},
    weapon::{MagazineSize, ModeShots},
};

#[test]
fn magazine_clamps_request_to_magazine_size() {
    let size = MagazineSize::new(12);

    let over = Magazine::new(LoadedRounds::new(100), size, RELOAD_TU);
    assert_eq!(
        *over.rounds(),
        *size,
        "a request above capacity clamps to MagazineSize"
    );

    let within = Magazine::new(LoadedRounds::new(5), size, RELOAD_TU);
    assert_eq!(
        *within.rounds(),
        5,
        "a request within capacity is preserved exactly"
    );

    let exact = Magazine::new(LoadedRounds::new(12), size, RELOAD_TU);
    assert_eq!(
        *exact.rounds(),
        *size,
        "a request equal to capacity is preserved"
    );

    let full = Magazine::loaded(size, RELOAD_TU);
    assert_eq!(*full.rounds(), *size, "loaded() fills to MagazineSize");
    assert!(*full.is_full(), "loaded() is full");
}

#[test]
fn set_size_clamps_rounds_to_the_new_capacity() {
    let size = MagazineSize::new(20);
    let smaller = MagazineSize::new(6);
    let larger = MagazineSize::new(30);

    let mut mag = Magazine::loaded(size, RELOAD_TU);
    mag.set_size(smaller);
    assert_eq!(
        *mag.rounds(),
        *smaller,
        "shrinking capacity clamps the loaded rounds down to the new size"
    );
    assert!(
        *mag.is_full(),
        "the clamped magazine is full at the new capacity"
    );

    mag.set_size(larger);
    assert_eq!(
        *mag.rounds(),
        *smaller,
        "growing capacity leaves the loaded rounds where they were"
    );
    assert!(
        !*mag.is_full(),
        "growing capacity does not hand out rounds, so the magazine is no longer full"
    );
}

#[test]
fn spend_round_is_saturating_on_empty_and_decrements_exactly() {
    let size = MagazineSize::new(30);

    let mut empty = Magazine::new(LoadedRounds::new(0), size, RELOAD_TU);
    empty.spend_round();
    assert_eq!(
        *empty.rounds(),
        0,
        "spending a round from an empty magazine stays 0"
    );

    let mut loaded = Magazine::new(LoadedRounds::new(3), size, RELOAD_TU);
    loaded.spend_round();
    assert_eq!(
        *loaded.rounds(),
        2,
        "spending one round drops the count by exactly 1"
    );
    loaded.spend_round();
    assert_eq!(*loaded.rounds(), 1, "and again");
    loaded.spend_round();
    assert_eq!(*loaded.rounds(), 0, "down to empty");
    loaded.spend_round();
    assert_eq!(
        *loaded.rounds(),
        0,
        "and the empty boundary still floors at 0"
    );
}

#[test]
fn refill_tops_loaded_rounds_to_capacity() {
    let size = MagazineSize::new(30);

    let mut depleted = Magazine::new(LoadedRounds::new(7), size, RELOAD_TU);
    assert!(!*depleted.is_full(), "precondition: not full");
    depleted.refill();
    assert_eq!(
        *depleted.rounds(),
        *size,
        "refill tops the loaded count to the capacity"
    );
    assert!(*depleted.is_full(), "after refill the magazine is full");

    let mut empty = Magazine::new(LoadedRounds::new(0), size, RELOAD_TU);
    empty.refill();
    assert_eq!(*empty.rounds(), *size, "refill fills an empty magazine");

    let mut full = Magazine::loaded(size, RELOAD_TU);
    full.refill();
    assert_eq!(
        *full.rounds(),
        *size,
        "refilling a full magazine leaves it full"
    );
}

#[test]
fn clamp_burst_bounds_shots_to_rounds_left() {
    let size = MagazineSize::new(30);

    let low = Magazine::new(LoadedRounds::new(2), size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &low),
        2,
        "a burst is clamped to the rounds actually loaded"
    );

    let full = Magazine::new(LoadedRounds::new(10), size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &full),
        5,
        "within ammo, the mode's full shot count passes through"
    );

    let empty = Magazine::new(LoadedRounds::new(0), size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &empty),
        0,
        "an empty magazine fires zero rounds"
    );
}

#[test]
fn set_reload_tu_overwrites_the_construction_cost() {
    let mut mag = Magazine::loaded(MagazineSize::new(12), RELOAD_TU);

    let new_cost = ReloadTu::new(7);
    mag.set_reload_tu(new_cost);
    assert_eq!(
        *mag.reload_tu(),
        *new_cost,
        "set_reload_tu writes the new cost; 12 is the construction cost and 0 the default, so \
         either value here means the write was dropped"
    );
}
