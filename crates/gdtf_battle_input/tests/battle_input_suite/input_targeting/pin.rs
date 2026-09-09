//! Pin: inspect/pin selection against fire mode and shooter state.
use bevy::{input::ButtonInput, prelude::*};
use cobalt_test_utils::{clear_mouse, press_left, unwatched_asset_plugin};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectMode, InspectTarget, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    occupancy::TerrainKind,
    prelude::{
        BattleInProgress, Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Tu,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        FireMode, FireModeSpec, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};

const PLAYER_FACTION: Faction = Faction::new(0);
const ENEMY_FACTION: Faction = Faction::new(1);
const LEVEL: Level = Level::new(0);

fn pin_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        unwatched_asset_plugin(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

const fn spec() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            FireMode::new(vec![spec()]),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

fn place_player_ganger(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    enemy
}

fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
}

fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut()
        .resource_mut::<InspectTarget>()
        .set_hovered(cell);
}

fn set_selection(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(SelectedShooter::new(entity));
}

fn pinned(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::pinned)
}

fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

fn effective(app: &App) -> Option<InspectMode> {
    app.world()
        .get_resource::<InspectTarget>()
        .map(InspectTarget::effective)
}

#[test]
fn clicking_cover_pins_that_cell() {
    let mut app = pin_app();
    let cell = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        pinned(&app),
        Some(cell),
        "clicking a cover cell must pin the inspect panel on it",
    );
}

#[test]
fn clicking_an_enemy_pins_that_cell() {
    let mut app = pin_app();
    let cell = CellLevel::new(Cell::new(3, 9), LEVEL);
    place_enemy(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        pinned(&app),
        Some(cell),
        "clicking an enemy fighter must pin the inspect panel on its cell",
    );
}

#[test]
fn clicking_an_empty_tile_unpins_and_hover_resumes() {
    let mut app = pin_app();

    let cover = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cover);
    set_hovered(&mut app, Some(cover));
    press_left(&mut app);
    app.update();
    assert_eq!(
        pinned(&app),
        Some(cover),
        "precondition: the cover is pinned"
    );

    let empty = CellLevel::new(Cell::new(1, 1), LEVEL);
    set_hovered(&mut app, Some(empty));
    press_left(&mut app);
    app.update();

    assert_eq!(pinned(&app), None, "clicking an empty tile must unpin");
    assert!(
        matches!(effective(&app), Some(InspectMode::Hovered(_))),
        "after unpin the panel follows hover (no longer Pinned)",
    );
}

#[test]
fn clicking_own_ganger_selects_and_keeps_the_pin() {
    let mut app = pin_app();

    let enemy_cell = CellLevel::new(Cell::new(3, 9), LEVEL);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));
    press_left(&mut app);
    app.update();
    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "precondition: the enemy is pinned",
    );

    let own_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let own = place_player_ganger(&mut app, own_cell);
    set_hovered(&mut app, Some(own_cell));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(own),
        "clicking your own ganger must SELECT it (the existing effect)",
    );
    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "selecting your own ganger must KEEP the pin (not clear it)",
    );
}

#[test]
fn clicking_to_fire_keeps_the_pin() {
    let mut app = pin_app();

    let shooter_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
    let shooter = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, shooter);

    let enemy_cell = CellLevel::new(Cell::new(5, 10), LEVEL);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "a fire click must leave the panel pinned on the enemy (never unpin)",
    );
}

#[test]
fn hover_does_not_change_the_panel_while_pinned() {
    let mut app = pin_app();

    let cover = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cover);
    set_hovered(&mut app, Some(cover));
    press_left(&mut app);
    app.update();
    assert_eq!(
        effective(&app),
        Some(InspectMode::Pinned(cover)),
        "precondition: the panel is pinned on the cover cell",
    );

    clear_mouse(&mut app);

    let elsewhere = CellLevel::new(Cell::new(30, 30), LEVEL);
    assert_ne!(elsewhere, cover, "the cursor moved off the pinned cell");
    set_hovered(&mut app, Some(elsewhere));
    app.update();

    assert_eq!(
        effective(&app),
        Some(InspectMode::Pinned(cover)),
        "hover must NOT change the panel's effective target while pinned",
    );
}
