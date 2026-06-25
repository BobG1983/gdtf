use super::{GroundDamage, SlabState, SurfaceGrid};
use crate::metric::{Cell, CellLevel, Level};

fn slab_key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// A no-op stand-in for an E1.6 occupancy update tick. The point of the C7(a)
/// test is that running a tick does NOT rebuild or touch the persistent surface
/// grid — the occupancy grid (GTW-156) is rebuilt per shot, the surface grid is
/// not. With the occupancy grid not built yet, this no-op is a faithful
/// stand-in: it takes the grid by shared ref and changes nothing.
fn occupancy_update_tick(_grid: &SurfaceGrid) {
    // Intentionally empty: an occupancy tick must not mutate the surface grid.
}

/// C7(a) — slab destruction is PERMANENT across an occupancy tick, and re-setting
/// it `Present`/`Absent` afterwards does NOT revert it.
///
/// Destroys a slab, runs a no-op occupancy update tick, and asserts the slab is
/// still `Destroyed`; then attempts `set_slab(Present)` and `set_slab(Absent)`
/// and asserts both are rejected (the slab stays `Destroyed`). This pins the C2
/// / C4 permanence invariant — a structural rule the acceptance criteria require.
#[test]
fn destroyed_slab_stays_destroyed_across_tick_and_resets() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(3, 4, 1);

    // Authored present, then destroyed.
    grid.set_slab(k, SlabState::Present);
    assert_eq!(grid.slab_state(&k), SlabState::Present);
    grid.destroy_slab(k);
    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);

    // An occupancy update tick (no-op stand-in for GTW-156) must NOT rebuild or
    // touch the persistent surface grid.
    occupancy_update_tick(&grid);
    assert_eq!(
        grid.slab_state(&k),
        SlabState::Destroyed,
        "a destroyed slab must survive an occupancy tick (the grid is not rebuilt)",
    );

    // Attempting to revert via set_slab is a no-op — destruction is permanent.
    grid.set_slab(k, SlabState::Present);
    assert_eq!(
        grid.slab_state(&k),
        SlabState::Destroyed,
        "set_slab(Present) must NOT revert a destroyed slab",
    );
    grid.set_slab(k, SlabState::Absent);
    assert_eq!(
        grid.slab_state(&k),
        SlabState::Destroyed,
        "set_slab(Absent) must NOT revert a destroyed slab",
    );
}

/// `destroy_slab` is idempotent and works even on a never-authored (`Absent`)
/// key — a slab record can be smashed regardless of its prior state, and a repeat
/// destroy is a harmless no-op (still `Destroyed`).
#[test]
fn destroy_slab_is_idempotent_and_works_on_absent() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(0, 0, 2);

    // Never authored Present — reads Absent by lazy default.
    assert_eq!(grid.slab_state(&k), SlabState::Absent);

    grid.destroy_slab(k);
    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
    // Repeat destroy: still Destroyed (idempotent).
    grid.destroy_slab(k);
    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
}

/// An absent slab key reads `Absent`, and an authored `Present`/`Absent` slab
/// (not yet destroyed) CAN still be re-set — the permanence guard only locks
/// `Destroyed`, not the pre-destruction authoring states.
#[test]
fn set_slab_freely_mutates_until_destroyed() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(7, 7, 0);

    assert_eq!(grid.slab_state(&k), SlabState::Absent);
    grid.set_slab(k, SlabState::Present);
    assert_eq!(grid.slab_state(&k), SlabState::Present);
    // Present → Absent is allowed (authoring), since it is not yet Destroyed.
    grid.set_slab(k, SlabState::Absent);
    assert_eq!(grid.slab_state(&k), SlabState::Absent);
}

/// C7(b) — ground damage is MONOTONIC: accruing twice yields the SUM.
///
/// Applies two arbitrary damage amounts to the same cell and asserts the stored
/// total equals their sum. Arbitrary magnitudes — the invariant is "the total is
/// the sum of accruals", not a specific number (not brittle).
#[test]
fn ground_damage_accrues_to_the_sum() {
    let mut grid = SurfaceGrid::new();
    let cell = Cell::new(5, 6);

    // Untouched cell reads zero.
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(0));

    let first = grid.accrue_ground_damage(cell, GroundDamage::new(13));
    assert_eq!(first, GroundDamage::new(13), "first accrual is the amount");
    let second = grid.accrue_ground_damage(cell, GroundDamage::new(9));
    assert_eq!(
        second,
        GroundDamage::new(22),
        "two accruals must total their sum (13 + 9)",
    );
    // And the stored total matches the returned running total.
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(22));
}

/// C7(b) the other side — a DECREASE is rejected (no-op): the guarded
/// `set_ground_damage` refuses any value below the current total, and there is no
/// additive path that can lower it.
///
/// Accrues damage, then attempts to set a lower total and asserts it is rejected
/// (returns `false`) and the stored total is unchanged; a value at-or-above the
/// current total is accepted. Pins the C4 monotonic invariant.
#[test]
fn ground_damage_decrease_is_rejected() {
    let mut grid = SurfaceGrid::new();
    let cell = Cell::new(2, 2);

    grid.accrue_ground_damage(cell, GroundDamage::new(40));
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(40));

    // A decrease attempt is rejected and leaves the total untouched.
    let lowered = grid.set_ground_damage(cell, GroundDamage::new(10));
    assert!(
        !lowered,
        "setting a value below the current total must be rejected"
    );
    assert_eq!(
        grid.ground_damage(&cell),
        GroundDamage::new(40),
        "a rejected decrease must NOT change the stored total",
    );

    // A non-decreasing set is accepted (it never lowers the accumulator).
    let raised = grid.set_ground_damage(cell, GroundDamage::new(55));
    assert!(
        raised,
        "setting a value at-or-above the current total is accepted"
    );
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(55));
}

/// `GroundDamage::accrue` saturates at `u32::MAX` rather than overflowing —
/// monotonicity is preserved at the ceiling (the total never wraps to a smaller
/// value).
#[test]
fn ground_damage_accrue_saturates() {
    let near_max = GroundDamage::new(u32::MAX - 1);
    let total = near_max.accrue(GroundDamage::new(10));
    assert_eq!(
        total,
        GroundDamage::new(u32::MAX),
        "accrual must saturate at u32::MAX, never wrap below the current total",
    );
}

/// The ground/slab maps are independent: a slab key and a ground key do not
/// collide, and the surface grid carries BOTH facts side by side.
#[test]
fn slab_and_ground_are_independent() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(1, 1, 3);
    let cell = Cell::new(1, 1);

    grid.destroy_slab(k);
    grid.accrue_ground_damage(cell, GroundDamage::new(7));

    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(7));
}

/// The `GroundDamage` derived [`Deref`] reaches its inner `u32`, and
/// `SlabState::is_destroyed` reports the terminal state. Arbitrary literals — the
/// Deref/predicate mechanism, not a magnitude.
#[test]
fn surface_newtypes_expose_inner() {
    assert_eq!(*GroundDamage::new(99), 99u32);
    assert!(SlabState::Destroyed.is_destroyed());
    assert!(!SlabState::Present.is_destroyed());
    assert!(!SlabState::Absent.is_destroyed());
}
