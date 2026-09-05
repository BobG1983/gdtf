//! HARNESS NOTE (deviation from the ticket's "use `MinimalTestAppBuilder`"): the sim crate
use bevy::{app::App, asset::AssetPlugin, prelude::MinimalPlugins, scene::ScenePlugin};
use gdtf_battle_sim::{
    battle::{BattleInProgress, BattleSimPlugin, SetupBattleRequested, TeardownBattleRequested},
    entity::TerrainPieceKind,
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{Faction, LifeState, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    visibility::SquadVisibility,
};

const SEED: u64 = 0x5A1C_AC75;

const PLAYER: u8 = 0;
const ENEMY: u8 = 1;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

const TEST_VIEW_RANGE: u16 = 3;

fn player_at() -> CellLevel {
    ground(5, 5)
}

fn enemy_at() -> CellLevel {
    ground(7, 5)
}

fn two_ganger_situation() -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_at())
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
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

fn squad(app: &App) -> Option<SquadVisibility> {
    app.world().get_resource::<SquadVisibility>().cloned()
}

fn player_count_at(app: &mut App, cell: CellLevel) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &Position)>();
    query
        .iter(world)
        .filter(|(faction, position)| ***faction == PLAYER && ***position == cell)
        .count()
}

#[test]
fn setup_inserts_and_fills_squad_visibility() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    assert!(
        squad(&app).is_some(),
        "setup_battle_on_request's Ok path must INSERT SquadVisibility",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "precondition: a successful setup inserts BattleInProgress",
    );

    let Some(fog) = squad(&app) else {
        unreachable!("asserted Some above");
    };
    assert!(
        *fog.is_cell_visible(&player_at()),
        "the spawn-time fog must mark the player ganger's own cell VISIBLE",
    );
    assert!(
        *fog.is_cell_explored(&player_at()),
        "a VISIBLE cell must also be EXPLORED (accrue's visible ⊆ explored invariant)",
    );
    assert!(
        *fog.is_cell_visible(&enemy_at()),
        "the spawn-time fog must see the enemy on clear ground within view range",
    );
}

#[test]
fn moving_player_reveals_new_cells_and_retains_explored() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *before.is_cell_visible(&enemy_at()),
        "precondition: the enemy cell is VISIBLE at spawn",
    );
    assert_eq!(
        player_count_at(&mut app, player_at()),
        1,
        "precondition: exactly one player ganger sits at the spawn cell",
    );

    let moved_to = ground(0, 0);
    {
        let world = app.world_mut();
        let mut query = world.query::<(&Faction, &mut Position)>();
        for (faction, mut position) in query.iter_mut(world) {
            if **faction == PLAYER {
                *position = Position::new(moved_to);
            }
        }
    }
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists across the battle");
    };
    assert!(
        *after.is_cell_visible(&moved_to),
        "after the move, the player's NEW cell must be VISIBLE (the writer recomputed)",
    );
    assert!(
        !*after.is_cell_visible(&enemy_at()),
        "after moving away, the enemy cell must DROP from VISIBLE",
    );
    assert!(
        *after.is_cell_explored(&enemy_at()),
        "a cell that left VISIBLE must STAY in EXPLORED (monotone accrual)",
    );
}

#[test]
fn downing_player_drops_its_fov() {
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(ground(8, 5))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            GangerSpawnBuilder::new()
                .at(ground(0, 0))
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_at())
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .build(),
        ])
        .build_with_gangs();

    let mut app = battle_app();
    drive_setup(&mut app, situation);

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *before.is_cell_visible(&enemy_at()),
        "precondition: the adjacent player A sees the enemy cell",
    );

    {
        let world = app.world_mut();
        let mut query = world.query::<(&Faction, &Position, &mut LifeState)>();
        for (faction, position, mut life) in query.iter_mut(world) {
            if **faction == PLAYER && **position == ground(8, 5) {
                *life = LifeState::Downed;
            }
        }
    }
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    assert!(
        !*after.is_cell_visible(&enemy_at()),
        "downing the only observer of the enemy must DROP its FOV (enemy cell leaves VISIBLE)",
    );
    assert!(
        *after.is_cell_explored(&enemy_at()),
        "the enemy cell stays EXPLORED after the observer is downed (monotone memory)",
    );
}

#[test]
fn cover_destroyed_triggers_a_recompute() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    let explored_before = before.explored_cells().count();

    app.world_mut().write_message(TerrainPieceDestroyed::new(
        ground(7, 5),
        TerrainPieceKind::Cover,
    ));
    app.update();
    app.update();

    let Some(after) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    assert!(
        after.explored_cells().count() >= explored_before,
        "a destroyed-cover recompute must keep EXPLORED monotone (never shrink)",
    );
    assert!(
        *after.is_cell_visible(&player_at()),
        "after the destroyed-cover recompute, the player still sees its own cell",
    );
}

#[test]
fn teardown_removes_squad_visibility() {
    let mut app = battle_app();
    drive_setup(&mut app, two_ganger_situation());
    assert!(
        squad(&app).is_some(),
        "precondition: SquadVisibility present after setup",
    );

    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    assert!(
        squad(&app).is_none(),
        "teardown_battle_on_request must REMOVE SquadVisibility (lifetime tracks BattleInProgress)",
    );
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "precondition: teardown also removes BattleInProgress (the shared lifetime)",
    );
}
