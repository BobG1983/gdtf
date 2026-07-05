//! Magazine state primitives — the clamp, the saturating spend, the refill, and
//! the burst clamp (mirrors `ammo.rs`).

use super::support::*;
use crate::{
    magazine::{Magazine, clamp_burst},
    weapon::{MagazineSize, ModeShots},
};

// AC1 — Magazine is clamped by the weapon's MagazineSize: a request above
// capacity clamps DOWN to capacity; a request within capacity is preserved
// exactly. (loaded() is the full-capacity convenience.)

#[test]
fn magazine_clamps_request_to_magazine_size() {
    let size = MagazineSize::new(12);

    // Asked for more than capacity → clamped to capacity.
    let over = Magazine::new(100, size, RELOAD_TU);
    assert_eq!(
        *over.rounds(),
        *size,
        "a request above capacity clamps to MagazineSize"
    );

    // Asked for within capacity → preserved exactly.
    let within = Magazine::new(5, size, RELOAD_TU);
    assert_eq!(
        *within.rounds(),
        5,
        "a request within capacity is preserved exactly"
    );

    // The exact-capacity request is preserved (the boundary).
    let exact = Magazine::new(12, size, RELOAD_TU);
    assert_eq!(
        *exact.rounds(),
        *size,
        "a request equal to capacity is preserved"
    );

    // loaded() is the full magazine.
    let full = Magazine::loaded(size, RELOAD_TU);
    assert_eq!(*full.rounds(), *size, "loaded() fills to MagazineSize");
    assert!(full.is_full(), "loaded() is full");
}

// AC2 — the per-round decrement is saturating: an empty (0-round) magazine
// stays 0 (no underflow/wrap), a non-empty one drops by EXACTLY one round.

#[test]
fn spend_round_is_saturating_on_empty_and_decrements_exactly() {
    let size = MagazineSize::new(30);

    // Empty magazine: spend_round must floor at 0, never wrap to ~65535.
    let mut empty = Magazine::new(0, size, RELOAD_TU);
    empty.spend_round();
    assert_eq!(
        *empty.rounds(),
        0,
        "spending a round from an empty magazine stays 0"
    );

    // Non-empty magazine: spend_round drops by exactly one.
    let mut loaded = Magazine::new(3, size, RELOAD_TU);
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

// refill — the reload primitive: tops the loaded count back to the capacity,
// regardless of how depleted it was, and is idempotent on an already-full mag.

#[test]
fn refill_tops_loaded_rounds_to_capacity() {
    let size = MagazineSize::new(30);

    // A depleted magazine refills to its full capacity.
    let mut depleted = Magazine::new(7, size, RELOAD_TU);
    assert!(!depleted.is_full(), "precondition: not full");
    depleted.refill();
    assert_eq!(
        *depleted.rounds(),
        *size,
        "refill tops the loaded count to the capacity"
    );
    assert!(depleted.is_full(), "after refill the magazine is full");

    // An empty magazine refills to full too.
    let mut empty = Magazine::new(0, size, RELOAD_TU);
    empty.refill();
    assert_eq!(*empty.rounds(), *size, "refill fills an empty magazine");

    // refill is idempotent on an already-full magazine.
    let mut full = Magazine::loaded(size, RELOAD_TU);
    full.refill();
    assert_eq!(
        *full.rounds(),
        *size,
        "refilling a full magazine leaves it full"
    );
}

// clamp_burst — the burst-clamp primitive: bounded by the rounds left, the
// mode's full count passes through within ammo.

#[test]
fn clamp_burst_bounds_shots_to_rounds_left() {
    let size = MagazineSize::new(30);

    // Fewer rounds than the burst wants → clamped to rounds left.
    let low = Magazine::new(2, size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &low),
        2,
        "a burst is clamped to the rounds actually loaded"
    );

    // Enough rounds → the mode's full shot count passes through.
    let full = Magazine::new(10, size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &full),
        5,
        "within ammo, the mode's full shot count passes through"
    );

    // Empty magazine → zero shots fire.
    let empty = Magazine::new(0, size, RELOAD_TU);
    assert_eq!(
        *clamp_burst(ModeShots::new(5), &empty),
        0,
        "an empty magazine fires zero rounds"
    );
}
