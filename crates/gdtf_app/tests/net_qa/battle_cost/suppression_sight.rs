//! The second way out of suppression over the wire: a walk that only breaks the line back to
//! the cell the fire came from is quoted legal, with no cover beside where it ends — and a walk
//! with nothing but the mover's own body on that line is still refused.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Suppressed, SuppressorCell, Tu},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, CellLevel},
};

use super::support::{a_reachable_cell_where, cost_calls, first_body, settle};
use crate::{
    battle_reads::a_player_ganger,
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// More time units than any walk costs, so nothing but the gate can refuse the quote.
const AMPLE_TU: Tu = Tu::new(u8::MAX);

/// How many steps of the walk the pin sits behind where the mover started.
const PIN_STEPS: i32 = 6;

/// How many steps along that line the wall stands.
const WALL_STEPS: i32 = 3;

/// What the case lined up: the mover, its walk, the cell the gate reads cover on, and the wall.
struct BrokenLine {
    actor:  Entity,
    dest:   CellLevel,
    beside: CellLevel,
    wall:   Vec<CellLevel>,
}

/// What the refusal case lined up: the mover, its one step, and the cell it steps off.
struct ClearLine {
    actor:  Entity,
    dest:   CellLevel,
    beside: CellLevel,
}

/// The step the walk takes, reversed: it points from the destination back past the start.
fn back_step(start: CellLevel, dest: CellLevel) -> (i32, i32) {
    (
        start.cell().x - dest.cell().x,
        start.cell().y - dest.cell().y,
    )
}

/// Where the fire came from: `steps` walks past the start, along the line the mover left.
fn along(start: CellLevel, dest: CellLevel, steps: i32) -> CellLevel {
    let (east, north) = back_step(start, dest);
    CellLevel::new(
        Cell::new(
            start.cell().x + east * steps,
            start.cell().y + north * steps,
        ),
        start.level(),
    )
}

/// The cell one step from `dest` toward `pin`, which is the one the cover branch reads.
fn beside(dest: CellLevel, pin: CellLevel) -> Option<CellLevel> {
    let step = Direction::from_cells(dest.cell(), pin.cell())?.cell_step();
    Some(CellLevel::new(
        Cell::new(dest.cell().x + step.x, dest.cell().y + step.y),
        dest.level(),
    ))
}

/// The nine cells the wall fills, so the ray cannot slip past a corner of it.
fn wall_cells(start: CellLevel, dest: CellLevel) -> Vec<CellLevel> {
    let middle = along(start, dest, WALL_STEPS);
    (-1..=1)
        .flat_map(|east| {
            (-1..=1).map(move |north| {
                CellLevel::new(
                    Cell::new(middle.cell().x + east, middle.cell().y + north),
                    middle.level(),
                )
            })
        })
        .collect()
}

/// A wall tall enough to break the line between the destination and the cell the fire came from.
const fn a_high_wall() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(100),
        HeightBand::High,
        ArmorProtection::new(50),
        ArmorHardness::new(50),
    )
}

/// Whether the cell the cover branch reads is bare, so only the sight branch can free this walk.
fn nothing_beside(app: &App, start: CellLevel, dest: CellLevel) -> bool {
    let Some(ledger) = app.world().get_resource::<CoverLedger>() else {
        return false;
    };
    beside(dest, along(start, dest, PIN_STEPS)).is_some_and(|cell| ledger.peek(&cell).is_none())
}

/// Pin the mover from behind, wall the line back to the pin, and leave its destination bare.
fn line_up(app: &mut App) -> Option<BrokenLine> {
    settle(app);
    let (actor, at) = a_player_ganger(app)?;
    let start = at.to_sim();
    let dest = a_reachable_cell_where(app, actor, |cell| {
        cell.level() == start.level()
            && cell.cell() != start.cell()
            && nothing_beside(app, start, *cell)
    })?;
    let pin = along(start, dest, PIN_STEPS);
    let wall = wall_cells(start, dest);
    let mut ledger = app.world_mut().get_resource_mut::<CoverLedger>()?;
    for cell in &wall {
        ledger.insert(*cell, a_high_wall());
    }
    let mut row = app.world_mut().get_entity_mut(actor).ok()?;
    row.insert(Suppressed::new(SuppressorCell::new(pin)));
    row.insert(AMPLE_TU);
    Some(BrokenLine {
        actor,
        dest,
        beside: beside(dest, pin)?,
        wall,
    })
}

/// Whether the cell is inside the battle grid, so a ray can be flown all the way to it.
fn on_the_grid(cell: CellLevel) -> bool {
    let inside =
        |value: i32, size: usize| i32::try_from(size).is_ok_and(|edge| (0..edge).contains(&value));
    inside(cell.cell().x, GRID_WIDTH) && inside(cell.cell().y, GRID_HEIGHT)
}

/// One step north, south, east or west, so the ray back to the pin crosses one cell only.
fn a_single_step(start: CellLevel, cell: CellLevel) -> bool {
    let (east, north) = (
        cell.cell().x - start.cell().x,
        cell.cell().y - start.cell().y,
    );
    cell.level() == start.level() && east.abs() + north.abs() == 1
}

/// Whether the cell the cover branch reads is bare, so only sight can free the walk.
fn bare_at(app: &App, cell: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .is_some_and(|ledger| ledger.peek(&cell).is_none())
}

/// Pin the mover from the cell right behind it, with open ground the whole way back.
fn line_up_clear(app: &mut App) -> Option<ClearLine> {
    settle(app);
    let (actor, at) = a_player_ganger(app)?;
    let start = at.to_sim();
    let dest = a_reachable_cell_where(app, actor, |cell| {
        a_single_step(start, *cell) && on_the_grid(along(start, *cell, 1)) && bare_at(app, start)
    })?;
    let mut row = app.world_mut().get_entity_mut(actor).ok()?;
    row.insert(Suppressed::new(SuppressorCell::new(along(start, dest, 1))));
    row.insert(AMPLE_TU);
    Some(ClearLine {
        actor,
        dest,
        beside: start,
    })
}

#[test]
fn a_suppressed_mover_with_only_its_own_body_on_the_line_is_still_refused() -> TestResult {
    let mut planned: Option<ClearLine> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(clear) = line_up_clear(app) else {
            return Vec::new();
        };
        let calls = cost_calls(
            clear.actor,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(clear.dest),
            }],
        );
        planned = Some(clear);
        calls
    })?;
    let Some(clear) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with a bare cell one step away and \
             another cell behind it on the map",
        ));
    };
    assert!(
        app.world()
            .get_entity(clear.actor)
            .is_ok_and(|row| row.contains::<Suppressed>()),
        "the mover must still be suppressed when the reply landed, or the case proves nothing",
    );
    assert!(
        bare_at(&app, clear.beside),
        "the cell the cover branch reads must be bare, or the refusal was never the sight \
         branch's to make",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::Suppressed),
        "the only thing standing between this destination and the cell the fire came from is the \
         mover itself, which does not hide it: the wire must refuse the walk: {body:?}",
    );
    assert!(
        !*body.legal,
        "a walk the suppression gate turns down is not quoted legal: {body:?}",
    );
    Ok(())
}

#[test]
fn a_suppressed_mover_that_only_loses_sight_of_the_shot_cell_is_quoted_a_legal_walk() -> TestResult
{
    let mut planned: Option<BrokenLine> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        let Some(broken) = line_up(app) else {
            return Vec::new();
        };
        let calls = cost_calls(
            broken.actor,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(broken.dest),
            }],
        );
        planned = Some(broken);
        calls
    })?;
    let Some(broken) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with a cell it can walk to that has no \
             cover beside it",
        ));
    };
    assert!(
        app.world()
            .get_entity(broken.actor)
            .is_ok_and(|row| row.contains::<Suppressed>()),
        "the mover must still be suppressed when the reply landed, or the case proves nothing",
    );
    let ledger = app.world().get_resource::<CoverLedger>();
    assert!(
        ledger.is_some_and(|ledger| ledger.peek(&broken.beside).is_none()),
        "the cell the cover branch reads must still be bare, or this walk was freed by cover \
         rather than by losing sight of the shot cell",
    );
    assert!(
        ledger.is_some_and(|ledger| broken
            .wall
            .iter()
            .all(|cell| ledger.peek(cell).is_some_and(|entry| !*entry.destroyed))),
        "the seeded wall must still stand when the reply landed, or the line back to the pin was \
         never broken",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal, None,
        "a suppressed mover that moves away and can no longer see where the fire came from \
         breaks away, so the wire must not refuse the walk: {body:?}",
    );
    assert!(
        *body.legal,
        "a walk the suppression gate allows must be quoted legal: {body:?}",
    );
    Ok(())
}
