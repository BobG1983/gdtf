use super::support::*;
use crate::pathfinder::Path;

fn on(x: i32, y: i32, storey: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(storey))
}

fn one_step(start: CellLevel, dest: CellLevel) -> Path {
    Path::new(vec![start, dest], vec![Tu::new(1)], Tu::new(1))
}

fn walled(at: CellLevel, band: HeightBand) -> CoverLedger {
    let mut ledger = CoverLedger::new();
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            band,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
        ),
    );
    ledger
}

// The cell the cover branch reads: one step from `dest` toward the cell the fire came from.
fn beside(dest: CellLevel, suppressor: CellLevel) -> Option<CellLevel> {
    let step = Direction::from_cells(dest.cell(), suppressor.cell())?.cell_step();
    Some(CellLevel::new(
        Cell::new(dest.cell().x + step.x, dest.cell().y + step.y),
        dest.level(),
    ))
}

// Whether that cell is bare, so only the sight branch can free the walk.
fn nothing_beside(cover: &CoverLedger, dest: CellLevel, suppressor: CellLevel) -> bool {
    beside(dest, suppressor).is_some_and(|cell| cover.peek(&cell).is_none())
}

fn verdict_in(
    terrain: &BareTerrain,
    mover: Entity,
    start: CellLevel,
    dest: CellLevel,
    suppressor: CellLevel,
    cover: &CoverLedger,
) -> MoveVerdict {
    let at = Position::new(start);
    let pool = Tu::new(100);
    let pinned = Suppressed::new(SuppressorCell::new(suppressor));
    can_move(
        Mover::new(mover, &at, &pool, &STANDING, &NORTH, Some(&pinned)),
        &dest,
        &one_step(start, dest),
        cover,
        &terrain.sight(),
    )
}

fn verdict(
    start: CellLevel,
    dest: CellLevel,
    suppressor: CellLevel,
    cover: &CoverLedger,
) -> MoveVerdict {
    let terrain = BareTerrain::new();
    verdict_in(&terrain, UNSPAWNED_MOVER, start, dest, suppressor, cover)
}

fn occupant_at(app: &App, at: CellLevel) -> Option<Entity> {
    app.world()
        .get_resource::<OccupancyGrid>()
        .and_then(|grid| grid.occupant(&at))
}

#[test]
fn a_farther_step_that_loses_sight_of_the_shot_cell_is_allowed_without_cover() {
    let (start, dest) = (on(1, 0, 0), on(0, 0, 0));
    let shot_cell = on(10, 0, 0);
    let cover = walled(on(5, 0, 0), HeightBand::High);

    assert!(
        nothing_beside(&cover, dest, shot_cell),
        "the cell beside the destination must be bare, or the cover branch frees this walk and \
         the case says nothing about sight",
    );
    assert_eq!(
        verdict(start, dest, shot_cell, &cover),
        MoveVerdict::Allowed,
        "a wall between the destination and the cell the fire came from hides the mover, which \
         is the second way out of suppression: no cover beside the destination is needed",
    );
}

#[test]
fn a_farther_step_still_in_sight_of_the_shot_cell_is_refused_without_cover() {
    let (start, dest) = (on(1, 0, 0), on(0, 0, 0));
    let shot_cell = on(10, 0, 0);

    assert_eq!(
        verdict(start, dest, shot_cell, &CoverLedger::new()),
        MoveVerdict::Suppressed,
        "the same step across open ground keeps the mover in view of the shot cell, so distance \
         on its own does not break the pin",
    );
}

#[test]
fn a_farther_step_behind_cover_is_allowed_while_the_shot_cell_is_still_in_sight() {
    let (start, dest) = (on(1, 0, 0), on(0, 0, 0));
    let shot_cell = on(10, 0, 0);
    let cover = walled(on(1, 0, 0), HeightBand::Mid);

    assert_eq!(
        verdict(start, dest, shot_cell, &cover),
        MoveVerdict::Allowed,
        "waist-high cover beside the destination frees the mover on its own, whether or not it \
         can still see where the fire came from",
    );
}

#[test]
fn a_climb_that_loses_sight_of_the_shot_cell_is_refused_because_height_is_not_distance() {
    let dest = on(0, 0, 1);
    let shot_cell = on(10, 0, 0);
    let cover = walled(on(5, 0, 1), HeightBand::High);

    assert_eq!(
        verdict(on(0, 0, 0), dest, shot_cell, &cover),
        MoveVerdict::Suppressed,
        "climbing a storey gains no ground across x and y, so losing sight up there is not a \
         break-away",
    );
    assert_eq!(
        verdict(on(1, 0, 1), dest, shot_cell, &cover),
        MoveVerdict::Allowed,
        "the same destination behind the same wall IS a break-away once the walk also gains a \
         cell across x and y, which is what makes the case above a height refusal",
    );
}

#[test]
fn the_mover_does_not_hide_from_the_shot_cell_behind_its_own_body() {
    let mut world = World::new();
    let mover = world.spawn_empty().id();
    let (start, dest) = (on(1, 0, 0), on(0, 0, 0));
    let shot_cell = on(10, 0, 0);
    let empty = CoverLedger::new();
    let mut terrain = BareTerrain::new();
    terrain.stand(start, mover);

    assert_eq!(
        verdict_in(&terrain, mover, start, dest, shot_cell, &empty),
        MoveVerdict::Suppressed,
        "the mover stands on the line it is asked about until it walks, and a break-away is \
         judged from where it will be: its own body cannot be what hides it",
    );
}

#[test]
fn another_body_on_the_line_hides_the_mover_and_frees_the_walk() {
    let mut world = World::new();
    let mover = world.spawn_empty().id();
    let bystander = world.spawn_empty().id();
    let (start, dest) = (on(1, 0, 0), on(0, 0, 0));
    let shot_cell = on(10, 0, 0);
    let empty = CoverLedger::new();
    let mut terrain = BareTerrain::new();
    terrain.stand(start, mover);
    terrain.stand(on(5, 0, 0), bystander);

    assert!(
        nothing_beside(&empty, dest, shot_cell),
        "nothing may stand beside the destination, or the cover branch frees this walk",
    );
    assert_eq!(
        verdict_in(&terrain, mover, start, dest, shot_cell, &empty),
        MoveVerdict::Allowed,
        "the probe reads the bodies on the grid: someone else between the destination and the \
         shot cell breaks the line, which is the only difference from the case above",
    );
}

#[test]
fn a_dispatched_walk_across_open_ground_is_rejected_with_the_mover_on_the_grid() {
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);
    let start = on(10, 10, 0);
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);

    app.update();
    assert_eq!(
        occupant_at(&app, start),
        Some(actor),
        "the mover must be registered where it stands, or this case cannot tell whether its own \
         body breaks the line",
    );

    let dest = on(9, 10, 0);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|reject| reject.actor == actor && reject.reason == MoveRejection::Suppressed),
        "a step away across open ground stays in view of the shot cell, so the live sim must \
         refuse it Suppressed: {rejects:?}",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(start)),
        "a refused break-away leaves the mover where it stood",
    );
}

#[test]
fn a_dispatched_walk_that_only_loses_sight_of_the_shot_cell_is_taken() {
    let mut app = headless_app();
    app.add_plugins(OccupancyMaintenancePlugin);
    let start = on(10, 10, 0);
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 100);
    suppress_from(&mut app, actor, 20, 10);
    author_wall(&mut app, 14, 10);

    app.update();
    assert_eq!(
        occupant_at(&app, start),
        Some(actor),
        "the mover must be registered where it stands, or the wall is not what breaks the line",
    );

    let dest = on(9, 10, 0);
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    assert!(
        drain_rejects(&mut app).is_empty(),
        "a walk that gains ground and puts a wall between the mover and the shot cell breaks the \
         pin, with nothing beside the destination",
    );
    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "the freed mover takes the step",
    );
}
