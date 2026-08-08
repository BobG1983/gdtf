use super::support::*;
use crate::pathfinder::Path;

fn on(x: i32, y: i32, storey: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(storey))
}

fn one_step(start: CellLevel, dest: CellLevel) -> Path {
    Path::new(vec![start, dest], vec![Tu::new(1)], Tu::new(1))
}

fn cover_on(at: CellLevel) -> CoverLedger {
    let mut ledger = CoverLedger::new();
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::Mid,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
        ),
    );
    ledger
}

fn verdict(
    start: CellLevel,
    dest: CellLevel,
    suppressor: CellLevel,
    cover: &CoverLedger,
) -> MoveVerdict {
    let at = Position::new(start);
    let pool = Tu::new(100);
    let pinned = Suppressed::new(SuppressorCell::new(suppressor));
    can_move(
        Mover::new(&at, &pool, Some(&pinned)),
        &dest,
        &one_step(start, dest),
        cover,
    )
}

#[test]
fn can_move_allows_a_break_away_with_the_suppressor_one_storey_above() {
    let (start, dest) = (on(10, 10, 0), on(9, 10, 0));
    let suppressor = on(20, 10, 1);

    assert_eq!(
        verdict(start, dest, suppressor, &cover_on(on(10, 10, 0))),
        MoveVerdict::Allowed,
        "the cover that shields the mover sits on the storey it ends on, so a suppressor one \
         storey up does not change which storey the gate reads",
    );
}

#[test]
fn can_move_allows_a_break_away_with_the_suppressor_one_storey_below() {
    let (start, dest) = (on(10, 10, 1), on(9, 10, 1));
    let suppressor = on(20, 10, 0);

    assert_eq!(
        verdict(start, dest, suppressor, &cover_on(on(10, 10, 1))),
        MoveVerdict::Allowed,
        "a mover a storey above its suppressor is freed by cover on its own storey",
    );
}

#[test]
fn can_move_refuses_a_break_away_when_cover_sits_only_on_the_suppressors_storey() {
    let (start, dest) = (on(10, 10, 0), on(9, 10, 0));
    let suppressor = on(20, 10, 1);

    assert_eq!(
        verdict(start, dest, suppressor, &cover_on(on(10, 10, 1))),
        MoveVerdict::Suppressed,
        "cover on the suppressor's storey shields nobody on the mover's: the gate must read the \
         destination's storey and refuse",
    );
}

#[test]
fn can_move_refuses_a_move_that_only_climbs_a_storey() {
    let (start, dest) = (on(10, 10, 0), on(10, 10, 1));
    let suppressor = on(20, 10, 0);

    assert_eq!(
        verdict(start, dest, suppressor, &cover_on(on(11, 10, 1))),
        MoveVerdict::Suppressed,
        "height is not distance: a climb to cover gains no ground on the suppressor, so the gate \
         refuses it even with cover waiting on the storey above",
    );
}

#[test]
fn suppressed_move_away_from_a_suppressor_one_storey_above_is_accepted() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from_on(&mut app, actor, 20, 10, Level::new(1));
    author_cover_on(&mut app, 10, 10, Level::new(0));

    let dest = on(9, 10, 0);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "the dispatch must not turn down a break-away behind cover on the mover's own storey",
    );
    let movements = drain_movements(&mut app);
    assert!(
        movements
            .iter()
            .any(|m| m.actor == actor && m.to == Cell::new(9, 10)),
        "the dispatch must step the mover and announce a MovementOccurred: {movements:?}",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "the mover ends on the destination cell and storey",
    );
}

#[test]
fn suppressed_move_away_from_a_suppressor_one_storey_below_is_accepted() {
    let mut app = headless_app();
    let actor = spawn_move_actor_on(app.world_mut(), 10, 10, Level::new(1), 100);
    suppress_from_on(&mut app, actor, 20, 10, Level::new(0));
    author_cover_on(&mut app, 10, 10, Level::new(1));

    let dest = on(9, 10, 1);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "a mover a storey above its suppressor breaks away behind cover on its own storey",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "the mover walks along its own storey to the destination",
    );
}
