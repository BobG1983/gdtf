//! The enter gate reads the seat's rotated entry sides, and nothing else about reach.

use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, can_enter_emplacement, enter_emplacement_tu_cost},
    ganger::{Direction, Position, Tu},
    metric::{Cell, CellLevel},
    terrain::{
        emplacement::{EmplacementEntrySides, EmplacementFacing, EmplacementState},
        facing::TerrainFacing,
    },
    test_support::{SituationBuilder, emplacement_at},
    tuning::CombatTuning,
};

use super::harness::*;

/// The eight neighbours of a cell, as the offsets that name them.
const AROUND: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// The cell one `step` from `seat`, on the same level.
fn stepped(seat: CellLevel, step: Cell) -> Position {
    let (cell, level) = seat.split();
    Position::new(CellLevel::new(
        Cell::new(cell.x + step.x, cell.y + step.y),
        level,
    ))
}

/// Default tuning and a pool that covers the enter cost, so only the sides term can refuse.
fn ample() -> (CombatTuning, Tu) {
    let tuning = CombatTuning::default();
    let pool = Tu::new(100);
    assert!(
        *pool > *enter_emplacement_tu_cost(&tuning),
        "the fixture pool must cover the enter cost, or the afford term refuses instead of the \
         sides term",
    );
    (tuning, pool)
}

#[test]
fn a_seat_naming_one_side_admits_only_the_cell_that_side_names() {
    let (tuning, pool) = ample();
    let seat = ground(9, 9);
    let side = TerrainFacing::ALL[0];
    let sides = EmplacementEntrySides::new(vec![side]);
    // An absent facing reads as the default one, so the authored side stands unrotated.
    let admitted = stepped(seat, side.cell_step());

    assert!(
        *can_enter_emplacement(
            admitted,
            Position::new(seat),
            &EmplacementState::Vacant,
            Some(&sides),
            None,
            &pool,
            &tuning,
        ),
        "the actor standing on the one cell {side:?} names is let in",
    );

    for (east, north) in AROUND {
        let standing = stepped(seat, Cell::new(east, north));
        if standing == admitted {
            continue;
        }
        assert!(
            !*can_enter_emplacement(
                standing,
                Position::new(seat),
                &EmplacementState::Vacant,
                Some(&sides),
                None,
                &pool,
                &tuning,
            ),
            "a seat naming only {side:?} refuses the actor standing on its ({east}, {north}) \
             neighbour at {standing:?}",
        );
    }
}

#[test]
fn the_one_authored_side_admits_a_different_cell_under_each_facing() {
    let (tuning, pool) = ample();
    let seat = ground(12, 7);
    let sides = EmplacementEntrySides::new(vec![TerrainFacing::ALL[0]]);
    let Some(unturned) = TerrainFacing::ALL
        .iter()
        .position(|side| *side == TerrainFacing::default())
    else {
        unreachable!("the default cardinal is one of the four the ring holds");
    };
    let turned = (unturned + 1) % TerrainFacing::ALL.len();
    let unrotated = stepped(seat, TerrainFacing::ALL[0].cell_step());

    for (facing, expected) in [
        (TerrainFacing::ALL[unturned], TerrainFacing::ALL[0]),
        (TerrainFacing::ALL[turned], TerrainFacing::ALL[1]),
    ] {
        let admitted = stepped(seat, expected.cell_step());
        assert!(
            *can_enter_emplacement(
                admitted,
                Position::new(seat),
                &EmplacementState::Vacant,
                Some(&sides),
                Some(&EmplacementFacing::new(facing)),
                &pool,
                &tuning,
            ),
            "a seat facing {facing:?} turns its one authored side onto {expected:?}, so the \
             actor at {admitted:?} is let in — read unrotated the side would point at \
             {unrotated:?} instead",
        );
    }
}

#[test]
fn a_seat_naming_no_side_refuses_every_neighbour() {
    let (tuning, pool) = ample();
    let seat = ground(4, 11);
    let sides = EmplacementEntrySides::new(Vec::new());

    for (east, north) in AROUND {
        let standing = stepped(seat, Cell::new(east, north));
        assert!(
            !*can_enter_emplacement(
                standing,
                Position::new(seat),
                &EmplacementState::Vacant,
                Some(&sides),
                None,
                &pool,
                &tuning,
            ),
            "a seat naming an EMPTY side list is enterable from nowhere, including its \
             ({east}, {north}) neighbour at {standing:?}",
        );
    }
}

#[test]
fn a_diagonal_neighbour_cannot_enter_and_is_charged_nothing() {
    let (mut app, seed) = battle_app(0x5543_1183);
    let seat = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 4), Direction::East)])
        .with_scatter(emplacement_at(seat))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat);
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    let Some(tu_before) = tu_of(&app, actor) else {
        unreachable!("the spawned ganger carries Tu");
    };
    assert!(
        tu_before >= enter_tu(&app),
        "the actor's pool must cover the enter cost, or the refusal below comes from the afford \
         term rather than from the entry sides",
    );

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "a diagonal neighbour stands on no cardinal entry side of the seat, so the enter is \
         refused",
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "a refused enter records no occupant",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_before),
        "a refused (non-entry) enter spends NO TU",
    );
}
