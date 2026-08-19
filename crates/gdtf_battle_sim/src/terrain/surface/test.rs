use super::{GroundDamage, SlabState, SurfaceGrid};
use crate::metric::{Cell, CellLevel, Level};

fn slab_key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

fn occupancy_update_tick(_grid: &SurfaceGrid) {}

#[test]
fn destroyed_slab_stays_destroyed_across_tick_and_resets() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(3, 4, 1);

    grid.set_slab(k, SlabState::Present);
    assert_eq!(grid.slab_state(&k), SlabState::Present);
    grid.destroy_slab(k);
    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);

    occupancy_update_tick(&grid);
    assert_eq!(
        grid.slab_state(&k),
        SlabState::Destroyed,
        "a destroyed slab must survive an occupancy tick (the grid is not rebuilt)",
    );

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

#[test]
fn destroy_slab_only_marks_a_present_slab() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(0, 0, 2);

    assert_eq!(grid.slab_state(&k), SlabState::Absent);

    grid.destroy_slab(k);
    assert_eq!(
        grid.slab_state(&k),
        SlabState::Absent,
        "destroying bare ground must write nothing; found {:?}",
        grid.slab_state(&k),
    );

    let k2 = slab_key(0, 1, 2);
    grid.set_slab(k2, SlabState::Present);
    grid.destroy_slab(k2);
    assert_eq!(grid.slab_state(&k2), SlabState::Destroyed);
    grid.destroy_slab(k2);
    assert_eq!(
        grid.slab_state(&k2),
        SlabState::Destroyed,
        "a second destroy on a destroyed slab must leave it Destroyed; found {:?}",
        grid.slab_state(&k2),
    );
}

#[test]
fn set_slab_freely_mutates_until_destroyed() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(7, 7, 0);

    assert_eq!(grid.slab_state(&k), SlabState::Absent);
    grid.set_slab(k, SlabState::Present);
    assert_eq!(grid.slab_state(&k), SlabState::Present);
    grid.set_slab(k, SlabState::Absent);
    assert_eq!(grid.slab_state(&k), SlabState::Absent);
}

#[test]
fn ground_damage_accrues_to_the_sum() {
    let mut grid = SurfaceGrid::new();
    let cell = Cell::new(5, 6);

    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(0));

    let first = grid.accrue_ground_damage(cell, GroundDamage::new(13));
    assert_eq!(first, GroundDamage::new(13), "first accrual is the amount");
    let second = grid.accrue_ground_damage(cell, GroundDamage::new(9));
    assert_eq!(
        second,
        GroundDamage::new(22),
        "two accruals must total their sum (13 + 9)",
    );
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(22));
}

#[test]
fn ground_damage_decrease_is_rejected() {
    let mut grid = SurfaceGrid::new();
    let cell = Cell::new(2, 2);

    grid.accrue_ground_damage(cell, GroundDamage::new(40));
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(40));

    let lowered = *grid.set_ground_damage(cell, GroundDamage::new(10));
    assert!(
        !lowered,
        "setting a value below the current total must be rejected"
    );
    assert_eq!(
        grid.ground_damage(&cell),
        GroundDamage::new(40),
        "a rejected decrease must NOT change the stored total",
    );

    let raised = *grid.set_ground_damage(cell, GroundDamage::new(55));
    assert!(
        raised,
        "setting a value at-or-above the current total is accepted"
    );
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(55));
}

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

#[test]
fn slab_and_ground_are_independent() {
    let mut grid = SurfaceGrid::new();
    let k = slab_key(1, 1, 3);
    let cell = Cell::new(1, 1);

    grid.set_slab(k, SlabState::Present);
    grid.destroy_slab(k);
    grid.accrue_ground_damage(cell, GroundDamage::new(7));

    assert_eq!(grid.slab_state(&k), SlabState::Destroyed);
    assert_eq!(grid.ground_damage(&cell), GroundDamage::new(7));
}

#[test]
fn surface_newtypes_expose_inner() {
    assert_eq!(*GroundDamage::new(99), 99u32);
    assert!(*SlabState::Destroyed.is_destroyed());
    assert!(!*SlabState::Present.is_destroyed());
    assert!(!*SlabState::Absent.is_destroyed());
}
