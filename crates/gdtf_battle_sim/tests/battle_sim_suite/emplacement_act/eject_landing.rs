//! Where a destroyed emplacement sets its gunner down when its own cell is taken.

use bevy::app::App;
use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::{Direction, Position},
    metric::{Cell, CellLevel, Level},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    test_support::test_pieces,
};

use super::harness::*;

/// The seed every case drives, so a rerun differs only in what the case writes.
const SEED: u64 = 0x5543_1308;

/// The act writes the destroyed message on the first update; the ejection reads it on the second.
fn settle_destruction(app: &mut App) {
    app.update();
    app.update();
}

/// The two free cells the ring-order case leaves open, differing only in `y`.
fn lower_free_cell() -> CellLevel {
    ground(6, 3)
}

fn upper_free_cell() -> CellLevel {
    ground(6, 7)
}

/// Every cell exactly `distance` from the seat, on the seat's level.
fn ring(distance: i32) -> Vec<CellLevel> {
    let seat = seat_cell();
    let mut cells = Vec::new();
    for dy in -distance..=distance {
        for dx in -distance..=distance {
            if dx.abs().max(dy.abs()) != distance {
                continue;
            }
            cells.push(CellLevel::new(
                Cell::new(seat.x + dx, seat.y + dy),
                seat.level(),
            ));
        }
    }
    cells
}

/// Path-block every in-bounds cell on the ground level, so no ring holds a free cell.
fn block_the_whole_level(app: &mut App) {
    let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
        unreachable!("battle setup inserts the occupancy grid this case blocks");
    };
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                continue;
            };
            grid.set_path_blocking(CellLevel::new(Cell::new(x, y), Level::new(0)));
        }
    }
}

#[test]
fn a_blocking_successor_pushes_the_gunner_off_the_mounts_own_cell() {
    let (mut app, seed) = battle_app(SEED);
    let attacker_cell = ground(6, 4);
    let bystander_cell = ground(10, 10);
    let seat = manned_seat_of(
        &mut app,
        seed,
        vec![
            player_at(attacker_cell, Direction::South),
            player_at(bystander_cell, Direction::East),
        ],
        test_pieces::EMPLACEMENT_LEAVING_WALL,
        Vec::new(),
    );
    let attacker = player_on(&mut app, attacker_cell);
    let bystander = player_on(&mut app, bystander_cell);
    let Some(mut position) = app.world_mut().get_mut::<Position>(bystander) else {
        unreachable!("every seeded ganger carries a Position");
    };
    *position = Position::new(entry_cell());

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, seat_cell()));
    settle_destruction(&mut app);

    assert!(
        app.world().get_entity(seat.emplacement).is_err(),
        "PRECONDITION: the smash must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    let landing = pos_of(&app, seat.occupant);
    assert_ne!(
        landing,
        Some(seat_cell()),
        "the successor wall stands in the seat, so the scan passes distance 0 rather than \
         setting the gunner down under it",
    );
    assert_ne!(
        landing,
        Some(entry_cell()),
        "the bystander holds the entered-from cell, so the gunner does not go back to it",
    );
}

#[test]
fn the_scan_takes_the_lower_y_of_two_free_cells_two_rings_out() {
    let (mut app, seed) = battle_app(SEED);
    let attacker_cell = ground(6, 4);
    let bystander_cell = ground(10, 10);
    let mut walls: Vec<CellLevel> = ring(1)
        .into_iter()
        .filter(|at| *at != entry_cell() && *at != attacker_cell)
        .collect();
    walls.extend(
        ring(2)
            .into_iter()
            .filter(|at| *at != lower_free_cell() && *at != upper_free_cell()),
    );
    let seat = manned_seat_of(
        &mut app,
        seed,
        vec![
            player_at(attacker_cell, Direction::South),
            player_at(bystander_cell, Direction::East),
        ],
        test_pieces::EMPLACEMENT_LEAVING_WALL,
        walls,
    );
    let bystander = player_on(&mut app, bystander_cell);
    let Some(mut position) = app.world_mut().get_mut::<Position>(bystander) else {
        unreachable!("every seeded ganger carries a Position");
    };
    *position = Position::new(entry_cell());
    let attacker = player_on(&mut app, attacker_cell);

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, seat_cell()));
    settle_destruction(&mut app);

    assert!(
        app.world().get_entity(seat.emplacement).is_err(),
        "PRECONDITION: the smash must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    assert_eq!(
        pos_of(&app, seat.occupant),
        Some(lower_free_cell()),
        "the scan reaches the second ring and takes the lower-y cell first; it set the gunner \
         down on {:?}",
        pos_of(&app, seat.occupant),
    );
}

#[test]
fn a_gunner_with_nowhere_free_to_go_is_left_on_the_mounts_own_cell() {
    let (mut app, seed) = battle_app(SEED);
    let attacker_cell = ground(6, 4);
    let seat = manned_seat_of(
        &mut app,
        seed,
        vec![player_at(attacker_cell, Direction::South)],
        test_pieces::EMPLACEMENT_LEAVING_WALL,
        Vec::new(),
    );
    let attacker = player_on(&mut app, attacker_cell);
    block_the_whole_level(&mut app);

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, seat_cell()));
    settle_destruction(&mut app);

    assert!(
        app.world().get_entity(seat.emplacement).is_err(),
        "PRECONDITION: the smash must fell the mount at {:?}, or nothing below is about a \
         destroyed emplacement",
        seat_cell(),
    );
    assert_ejected(&mut app, &seat, seat_cell());
}
