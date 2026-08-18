use bevy::prelude::{App, Entity, IntoScheduleConfigs, Update, World};

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, drain_moves, drain_opens, ground,
    place_occupant, spawn_combatant, tu_of,
};
use crate::{
    acts::OpenDoorRequested,
    cover::HeightBand,
    ganger::{Direction, Position, Tu},
    occupancy::{GRID_HEIGHT, project_path_blocking},
    occupancy_sync::{OccupancyMaintenancePlugin, SimSystems},
    openable::{OpenState, OpenableBlocking, apply_openable_toggle},
    terrain::entity::{BlocksPathfinding, BlocksVision, TerrainCell},
};

const FRAME_CAP: usize = 160;
const WALL_X: i32 = 5;
const DOOR_Y: i32 = 5;

fn door_brain_app() -> App {
    let mut app = brain_app();
    app.add_plugins(OccupancyMaintenancePlugin);
    app.add_systems(
        Update,
        apply_openable_toggle
            .in_set(SimSystems::Simulate)
            .before(project_path_blocking),
    );
    app
}

fn seal_column(world: &mut World, door_y: i32) -> Entity {
    let mut door = Entity::PLACEHOLDER;
    for y in 0..i32::try_from(GRID_HEIGHT).unwrap_or(0) {
        let at = ground(WALL_X, y);
        if y == door_y {
            door = world
                .spawn((
                    TerrainCell::new(at),
                    OpenState::Closed,
                    OpenableBlocking::new(HeightBand::High),
                    BlocksPathfinding,
                    BlocksVision::new(HeightBand::High),
                ))
                .id();
        } else {
            world.spawn((
                TerrainCell::new(at),
                BlocksPathfinding,
                BlocksVision::new(HeightBand::High),
            ));
        }
    }
    door
}

fn door_is_open(app: &App, door: Entity) -> bool {
    app.world()
        .get::<OpenState>(door)
        .is_some_and(|state| *state.is_open())
}

fn enemy_x(app: &App, enemy: Entity) -> i32 {
    app.world()
        .get::<Position>(enemy)
        .map_or(0, |position| position.x)
}

#[test]
fn adjacent_closed_door_opens_then_the_enemy_walks_through() {
    let mut app = door_brain_app();
    let door = seal_column(app.world_mut(), DOOR_Y);
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(4, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut opened = false;
    let mut walked_through = false;
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        if drain_opens(&mut app)
            .iter()
            .any(|open| open.actor == enemy && open.door == door)
        {
            opened = true;
        }
        if opened
            && (door_is_open(&app, door) && enemy_x(&app, enemy) > WALL_X
                || drain_moves(&mut app)
                    .iter()
                    .any(|step| step.actor == enemy && step.dest.x > WALL_X))
        {
            walked_through = true;
        }
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        opened,
        "an enemy stalled beside a closed door must emit OpenDoorRequested"
    );
    assert!(
        door_is_open(&app, door),
        "the real open-door dispatch must flip the door to Open"
    );
    assert!(
        walked_through,
        "after the door opens the enemy must walk through it toward the player"
    );
    assert!(
        returned,
        "the enemy turn must hand control back to the player within the cap"
    );
}

#[test]
fn reachable_door_is_walked_to_then_opened() {
    let mut app = door_brain_app();
    let door = seal_column(app.world_mut(), DOOR_Y);
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut walked_to_door = false;
    let mut opened = false;
    let mut left = false;
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        if drain_moves(&mut app).iter().any(|step| {
            step.actor == enemy && step.dest.x > 2 && step.dest.x <= WALL_X && step.dest.y == DOOR_Y
        }) {
            walked_to_door = true;
        }
        if drain_opens(&mut app)
            .iter()
            .any(|open| open.actor == enemy && open.door == door)
        {
            opened = true;
        }
        if opened
            && (enemy_x(&app, enemy) > WALL_X
                || drain_moves(&mut app)
                    .iter()
                    .any(|step| step.actor == enemy && step.dest.x > WALL_X))
        {
            left = true;
        }
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        walked_to_door,
        "a door on the room wall must be walked to before it is opened"
    );
    assert!(opened, "once adjacent, the enemy must open the door");
    assert!(left, "after opening, the enemy must leave the room");
    assert!(
        returned,
        "the enemy turn must hand control back to the player within the cap"
    );
}

#[test]
fn a_clear_advance_does_not_open_a_side_door() {
    let mut app = door_brain_app();
    app.world_mut().spawn((
        TerrainCell::new(ground(5, 40)),
        OpenState::Closed,
        OpenableBlocking::new(HeightBand::High),
        BlocksPathfinding,
        BlocksVision::new(HeightBand::High),
    ));
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut opens = Vec::new();
    let mut moves = Vec::new();
    let mut fires = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        opens.extend(drain_opens(&mut app));
        moves.extend(drain_moves(&mut app));
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        opens.is_empty(),
        "an enemy that can already advance must not emit OpenDoorRequested: {opens:?}"
    );
    assert!(
        moves
            .iter()
            .any(|step| step.actor == enemy && step.dest.x > 2),
        "the enemy must still step toward the player: {moves:?}"
    );
    assert!(
        fires.is_empty(),
        "the far player must not be fired on: {fires:?}"
    );
    assert!(returned, "the enemy turn must return to the player");
}

#[test]
fn an_adjacent_door_is_not_opened_when_the_pool_cannot_cover_it() {
    let mut app = door_brain_app();
    let door = seal_column(app.world_mut(), DOOR_Y);
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(4, 5),
        ENEMY,
        Direction::East,
        100,
        6,
    );
    app.world_mut().entity_mut(enemy).insert(Tu::new(3));
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let mut opens: Vec<OpenDoorRequested> = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        opens.extend(drain_opens(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        opens.is_empty(),
        "an enemy that cannot afford the open must not emit OpenDoorRequested: {opens:?}"
    );
    assert!(
        !door_is_open(&app, door),
        "the door must stay closed when the pool cannot cover the act"
    );
    assert_eq!(tu_of(&app, enemy), 3, "a refused open spends nothing");
    assert!(
        returned,
        "the enemy turn must end and hand control back to the player"
    );
}
