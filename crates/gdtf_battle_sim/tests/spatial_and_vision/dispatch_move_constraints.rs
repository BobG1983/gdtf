//! HARNESS NOTE (deviation from the ticket's "use `GdtfTestAppBuilder`"): the sim crate is
use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, Messages, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::{MoveRejected, MoveRejection, MoveRequested, MovementOccurred},
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{GangRegistry, Speed},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

const SEED: u64 = 0x5A1C_AC75;

const PLAYER: u8 = 0;

const TEST_VIEW_RANGE: u16 = 3;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn player_at() -> CellLevel {
    ground(5, 5)
}

fn adjacent_open() -> CellLevel {
    ground(6, 5)
}

fn far_unseen() -> CellLevel {
    ground(40, 40)
}

fn one_player_situation(speed: f32) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([GangerSpawnBuilder::new()
            .at(player_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .speed(Speed::new(speed))
            .build()])
        .build_with_gangs()
}

fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

fn player_entity_and_pos(app: &mut App) -> Option<(Entity, CellLevel)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction, &Position)>();
    query
        .iter(world)
        .find(|(_, faction, _)| ***faction == PLAYER)
        .map(|(entity, _, position)| (entity, **position))
}

fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

fn clear_signals(app: &mut App) {
    let _movements: usize = drain_movements(app).len();
    let _rejects: usize = drain_rejects(app).len();
}

#[test]
fn unreachable_teleport_is_rejected_with_no_move() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    clear_signals(&mut app);

    app.world_mut()
        .write_message(MoveRequested::new(actor, far_unseen()));
    app.update();
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Unreachable),
        "an unreachable teleport must emit MoveRejected(Unreachable): {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "an unreachable teleport announces NO MovementOccurred",
    );
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, before,
        "an unreachable teleport leaves the mover exactly where it was (no teleport)",
    );
}

#[test]
fn reachable_route_is_accepted_with_move_and_log() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    assert_eq!(
        before,
        player_at(),
        "precondition: the player starts at its spawn cell",
    );
    clear_signals(&mut app);

    let dest = adjacent_open();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();
    app.update();

    let movements = drain_movements(&mut app);
    assert!(
        movements.iter().any(|m| m.actor == actor),
        "a reachable affordable move must announce a MovementOccurred for the actor: {movements:?}",
    );
    assert!(
        drain_rejects(&mut app).is_empty(),
        "a reachable affordable move must emit NO MoveRejected",
    );
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, dest,
        "a reachable affordable move must step the mover to the destination",
    );
}

#[test]
fn unaffordable_route_is_rejected_with_no_partial_move() {
    let mut app = battle_app();
    drive_setup(&mut app, one_player_situation(20.0));

    let Some((actor, before)) = player_entity_and_pos(&mut app) else {
        unreachable!("setup spawns exactly one player ganger");
    };
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(actor) {
        *tu = Tu::new(0);
    }
    assert_eq!(
        before,
        player_at(),
        "precondition: the player starts at its spawn cell",
    );
    clear_signals(&mut app);

    let dest = adjacent_open();
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();
    app.update();

    let rejects = drain_rejects(&mut app);
    assert!(
        rejects
            .iter()
            .any(|r| r.actor == actor && r.reason == MoveRejection::Unaffordable),
        "an unaffordable route must emit MoveRejected(Unaffordable): {rejects:?}",
    );
    assert!(
        drain_movements(&mut app).is_empty(),
        "an unaffordable route announces NO MovementOccurred (no partial move)",
    );
    let Some((_, after)) = player_entity_and_pos(&mut app) else {
        unreachable!("the player ganger persists");
    };
    assert_eq!(
        after, before,
        "an unaffordable route leaves the mover exactly where it was (NO partial move)",
    );
}
