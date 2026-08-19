use super::support::*;
use crate::pathfinder::Path;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn one_step(start: CellLevel, dest: CellLevel) -> Path {
    Path::new(vec![start, dest], vec![Tu::new(1)], Tu::new(1))
}

fn cover_at(x: i32, y: i32) -> CoverLedger {
    let mut ledger = CoverLedger::new();
    ledger.insert(
        ground(x, y),
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::Mid,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
            TerrainPieceKind::Cover,
        ),
    );
    ledger
}

#[test]
fn can_move_refuses_a_suppressed_step_toward_the_suppressor() {
    let (start, dest) = (ground(10, 10), ground(11, 10));
    let at = Position::new(start);
    let pool = Tu::new(100);
    let pinned = Suppressed::new(SuppressorCell::new(ground(20, 10)));
    let terrain = BareTerrain::new();

    assert_eq!(
        can_move(
            Mover::new(
                UNSPAWNED_MOVER,
                &at,
                &pool,
                &STANDING,
                &NORTH,
                Some(&pinned)
            ),
            &dest,
            &one_step(start, dest),
            &cover_at(12, 10),
            &terrain.sight(),
        ),
        MoveVerdict::Suppressed,
        "the gate the dispatch and battle.cost share must refuse a step toward the suppressor, \
         however full the pool",
    );
}

#[test]
fn can_move_allows_a_suppressed_step_that_breaks_away_behind_cover() {
    let (start, dest) = (ground(10, 10), ground(9, 10));
    let at = Position::new(start);
    let pool = Tu::new(100);
    let pinned = Suppressed::new(SuppressorCell::new(ground(20, 10)));
    let terrain = BareTerrain::new();

    assert_eq!(
        can_move(
            Mover::new(
                UNSPAWNED_MOVER,
                &at,
                &pool,
                &STANDING,
                &NORTH,
                Some(&pinned)
            ),
            &dest,
            &one_step(start, dest),
            &cover_at(10, 10),
            &terrain.sight(),
        ),
        MoveVerdict::Allowed,
        "the same gate must allow a step that ends farther away and behind cover",
    );
}

#[test]
fn can_move_names_suppression_when_an_empty_pool_would_also_refuse() {
    let (start, dest) = (ground(10, 10), ground(11, 10));
    let at = Position::new(start);
    let empty = Tu::new(0);
    let pinned = Suppressed::new(SuppressorCell::new(ground(20, 10)));
    let terrain = BareTerrain::new();

    assert_eq!(
        can_move(
            Mover::new(
                UNSPAWNED_MOVER,
                &at,
                &empty,
                &STANDING,
                &NORTH,
                Some(&pinned)
            ),
            &dest,
            &one_step(start, dest),
            &cover_at(12, 10),
            &terrain.sight(),
        ),
        MoveVerdict::Suppressed,
        "suppression is judged before the pool: a mover who is both suppressed and short of TU \
         is refused Suppressed, which is what the dispatch rejects it with",
    );
}

#[test]
fn suppressed_move_toward_suppressor_is_rejected() {
    let mut app = headless_app();
    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    author_cover(&mut app, 12, 10);

    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Suppressed),
        "a suppressed mover stepping toward the suppressor must be rejected Suppressed: {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "a rejected suppressed move takes NO step — it announces no MovementOccurred",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(start)),
        "a rejected suppressed move leaves the mover exactly where it was (no partial step)",
    );
}

#[test]
fn suppressed_move_to_exposed_farther_cell_is_rejected() {
    let mut app = headless_app();
    let start = CellLevel::new(Cell::new(10, 10), Level::new(0));
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);

    let dest = CellLevel::new(Cell::new(9, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Suppressed),
        "a suppressed mover to a farther-but-exposed cell must be rejected Suppressed: {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "a rejected suppressed move takes NO step — it announces no MovementOccurred",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(start)),
        "a rejected suppressed move leaves the mover exactly where it was (no partial step)",
    );
}

#[test]
fn suppressed_move_farther_behind_cover_is_accepted() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    author_cover(&mut app, 10, 10);

    let dest = CellLevel::new(Cell::new(9, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "a legal suppressed retreat (farther + behind cover) must NOT be rejected",
    );
    let movements = drain_movements(&mut app);
    assert!(
        movements
            .iter()
            .any(|m| m.actor == actor && m.to == Cell::new(9, 10)),
        "a legal suppressed retreat must step the mover and announce a MovementOccurred: {movements:?}",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "a legal suppressed retreat steps the mover to the destination",
    );
}

#[test]
fn unsuppressed_move_with_same_geometry_is_unaffected() {
    let mut app = headless_app();
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);

    let dest = CellLevel::new(Cell::new(11, 10), Level::new(0));
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "an unsuppressed mover is never subject to the suppression gate — no MoveRejected",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "an unsuppressed mover steps freely to the destination (identity path)",
    );
}
