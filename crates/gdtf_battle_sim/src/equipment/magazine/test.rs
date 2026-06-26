//! Ammo-state + shared firing-guard proofs: the magazine clamp / saturating
//! spend / burst clamp, and the `can_fire` guard cases (AC1–AC8).

use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    injuries::HandsAvailable,
    magazine::{FireActor, Magazine, ReloadTu, can_fire, clamp_burst, in_bounds, mode_tu_cost},
    metric::{Cell, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    tuning::CombatTuning,
    weapon::{
        FireModeSpec, Handedness, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};

/// An arbitrary per-weapon reload cost the magazine fixtures carry — the magnitude is
/// tunable, so tests never assert it; they assert RELATIONS over the magazine state.
const RELOAD_TU: ReloadTu = ReloadTu::new(12);

/// A fire-mode spec with an arbitrary (non-pinned) TU% — the per-mode magnitude
/// is tuning, so tests never assert it; they assert RELATIONS over it.
const fn mode(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A loaded, affordable, alive, aim-flag-controllable actor over borrowed state
/// — the shared fixture the `can_fire` cases vary one field at a time. Wields a
/// `OneHanded` weapon with the uninjured two-hands default, so the GTW-443 hand-count
/// clause always passes here (the hand-count cases use [`hand_actor`] to vary it).
fn actor<'a>(
    life: &'a LifeState,
    tu: &'a Tu,
    tu_max: &'a TuMax,
    aiming: &'a Aiming,
    magazine: &'a Magazine,
) -> FireActor<'a> {
    FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness: Handedness::OneHanded,
        hands_available: HandsAvailable::default(),
    }
}

/// An alive, affordable, loaded actor with a chosen [`Handedness`] +
/// [`HandsAvailable`] — the GTW-443 hand-count fixture, varying only the two hand fields
/// (everything else passes `can_fire`).
fn hand_actor<'a>(
    life: &'a LifeState,
    tu: &'a Tu,
    tu_max: &'a TuMax,
    aiming: &'a Aiming,
    magazine: &'a Magazine,
    handedness: Handedness,
    hands_available: HandsAvailable,
) -> FireActor<'a> {
    FireActor {
        life,
        tu,
        tu_max,
        aiming,
        magazine,
        handedness,
        hands_available,
    }
}

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
    let magazine = Magazine::new(10, size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    assert!(
        can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let magazine = Magazine::new(10, size, RELOAD_TU);

    for life in [LifeState::Downed, LifeState::Dead] {
        let a = actor(&life, &tu, &tu_max, &aiming, &magazine);
        assert!(
            !can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let magazine = Magazine::new(10, size, RELOAD_TU);

    // The exact hip-fire charge for this mode.
    let charge = mode_tu_cost(&m, &tu_max, &aiming, &tuning);
    assert!(*charge > 0, "the mode charge must be positive for the test");

    // One TU short of the charge → cannot afford → false.
    let short = Tu::new(charge.saturating_sub(1));
    let a_short = actor(&life, &short, &tu_max, &aiming, &magazine);
    assert!(
        !can_fire(&a_short, &m, Cell::new(10, 10), Level::new(0), &tuning),
        "a pool one below the mode charge cannot fire"
    );

    // Exactly the charge → can afford → true (equality affords).
    let exact = Tu::new(*charge);
    let a_exact = actor(&life, &exact, &tu_max, &aiming, &magazine);
    assert!(
        can_fire(&a_exact, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let empty = Magazine::new(0, size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &empty);

    assert!(
        !can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
        in_bounds(Cell::new(0, 0), Level::new(0)),
        "the origin cell on storey 0 is in-bounds"
    );
    assert!(
        in_bounds(Cell::new(w - 1, h - 1), Level::new(MAX_LEVELS - 1)),
        "the top corner (GRID_WIDTH-1, GRID_HEIGHT-1, MAX_LEVELS-1) is in-bounds"
    );

    // x == GRID_WIDTH is one past the x extent → out.
    assert!(
        !in_bounds(Cell::new(w, 0), Level::new(0)),
        "x == GRID_WIDTH is out of bounds"
    );
    // y == GRID_HEIGHT is one past the y extent → out.
    assert!(
        !in_bounds(Cell::new(0, h), Level::new(0)),
        "y == GRID_HEIGHT is out of bounds"
    );
    // level == MAX_LEVELS is one past the z extent → out.
    assert!(
        !in_bounds(Cell::new(0, 0), Level::new(MAX_LEVELS)),
        "level == MAX_LEVELS is out of bounds"
    );
    // Any negative axis is out (a Cell wraps a signed IVec2).
    assert!(
        !in_bounds(Cell::new(-1, 0), Level::new(0)),
        "negative x is out of bounds"
    );
    assert!(
        !in_bounds(Cell::new(0, -1), Level::new(0)),
        "negative y is out of bounds"
    );
}

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
    let magazine = Magazine::new(10, size, RELOAD_TU);

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
            !can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let magazine = Magazine::new(10, size, RELOAD_TU);

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
        can_fire(&one_hand, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
        !can_fire(&no_hands, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let magazine = Magazine::new(10, size, RELOAD_TU);

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
        can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
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
    let magazine = Magazine::new(10, size, RELOAD_TU);

    let default_actor = actor(&life, &tu, &tu_max, &aiming, &magazine);
    assert!(
        can_fire(
            &default_actor,
            &m,
            Cell::new(10, 10),
            Level::new(0),
            &tuning
        ),
        "an uninjured (OneHanded, two-hands) actor passes the hand-count clause"
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
    let magazine = Magazine::new(10, size, RELOAD_TU);
    let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

    // can_fire's full input set is (actor, mode, cell, level, tuning) — no
    // has_los / visibility argument exists. The shooter passes on those inputs
    // alone; whether a target is "seen" is the presenter's fog gate, which the
    // model act never consults.
    assert!(
        can_fire(&a, &m, Cell::new(20, 20), Level::new(2), &tuning),
        "an alive, affordable, loaded, in-bounds shooter passes can_fire with no LOS input"
    );
}
